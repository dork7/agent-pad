//! The Core's local HTTP API.

use std::io::{self, Read};
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread::JoinHandle;

use percent_encoding::percent_decode_str;
use serde_json::json;
use tiny_http::{Header, Method, Request, Response, Server};

use crate::{hooks, ImplementError, RunError, State, TicketError};

pub(crate) struct Running {
    pub addr: SocketAddr,
    server: Arc<Server>,
    thread: Option<JoinHandle<()>>,
}

pub(crate) fn start(state: Arc<State>) -> io::Result<Running> {
    let server = Server::http("127.0.0.1:0").map_err(io::Error::other)?;
    let addr = server
        .server_addr()
        .to_ip()
        .ok_or_else(|| io::Error::other("server is not bound to an IP address"))?;
    let server = Arc::new(server);
    let thread = {
        let server = server.clone();
        std::thread::spawn(move || {
            for request in server.incoming_requests() {
                handle(&state, request);
            }
        })
    };
    Ok(Running {
        addr,
        server,
        thread: Some(thread),
    })
}

impl Drop for Running {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn handle(state: &State, mut request: Request) {
    let path = request.url().split('?').next().unwrap_or("").to_string();
    let response = match (request.method(), path.as_str()) {
        (Method::Get, "/skills") => skills_response(state),
        (Method::Get, "/harnesses") => harnesses_response(state),
        (Method::Get, "/sessions") => sessions_response(state),
        (Method::Get, path) if path.starts_with("/sessions/") => tickets_response(state, path)
            .with_header(Header::from_bytes("Cache-Control", "no-store").expect("valid header")),
        (Method::Post, path) if path.starts_with("/sessions/") && path.ends_with("/run") =>
            run_response(state, path, &mut request, None),
        (Method::Post, path) if path.starts_with("/sessions/") => implement_response(state, path),
        (Method::Post, "/hooks/claude") => {
            hooks::receive(state, &mut request, hooks::Harness::Claude)
        }
        (Method::Post, "/hooks/codex") => {
            hooks::receive(state, &mut request, hooks::Harness::Codex)
        }
        _ => Response::from_string("not found").with_status_code(404),
    };
    let _ = request.respond(response);
}

pub(crate) fn run_response(state: &State, path: &str, request: &mut Request, device: Option<&str>) -> Response<io::Cursor<Vec<u8>>> {
    let ["sessions", session, "run"] = path.split('/').filter(|part| !part.is_empty()).collect::<Vec<_>>()[..] else {
        return json_status(404, &json!({"error": "not found"}));
    };
    let Some(session) = decode_segment(session) else {
        return json_status(404, &json!({"error": "unknown session"}));
    };
    #[derive(serde::Deserialize)]
    struct Body { id: String }
    let mut body = String::new();
    let parsed = request.as_reader().take(64 * 1024).read_to_string(&mut body)
        .ok().and_then(|_| serde_json::from_str::<Body>(&body).ok());
    let Some(Body { id }) = parsed else {
        return json_status(400, &json!({"error": "expected skill id"}));
    };
    if device.is_some_and(|device| !state.skills_for_phone(device).iter().any(|skill| skill.id == id)) {
        return json_status(403, &json!({"error": "not in this phone's view"}));
    }
    match state.run_skill_in_session(&session, &id) {
        Ok(skill) => json(&json!({"running": skill.invocation})),
        Err(RunError::UnknownSession | RunError::UnknownSkill) => json_status(404, &json!({"error": "session or skill not found"})),
        Err(RunError::WrongHarness) => json_status(409, &json!({"error": "skill does not belong to this session's harness"})),
        Err(RunError::Blocked) => json_status(409, &json!({"error": "answer the session request first"})),
        Err(RunError::WindowNotFound) => json_status(409, &json!({"error": "session window was not found"})),
    }
}

pub(crate) fn skills_response(state: &State) -> Response<io::Cursor<Vec<u8>>> {
    json(&state.scan())
}

pub(crate) fn harnesses_response(state: &State) -> Response<io::Cursor<Vec<u8>>> {
    json(&state.harnesses())
}

pub(crate) fn sessions_response(state: &State) -> Response<io::Cursor<Vec<u8>>> {
    json(&state.list_sessions())
}

pub(crate) fn tickets_response(state: &State, path: &str) -> Response<io::Cursor<Vec<u8>>> {
    let not_found = || Response::from_string("not found").with_status_code(404);
    let Some((session, suffix)) = path
        .strip_prefix("/sessions/")
        .and_then(|path| path.split_once('/'))
    else {
        return not_found();
    };
    let Some(session_id) = decode_segment(session) else {
        return not_found();
    };
    let result = if suffix == "tickets" {
        state
            .pending_tickets(&session_id)
            .map(|tickets| json(&tickets))
    } else if let Some(ticket_id) = suffix.strip_prefix("tickets/").and_then(decode_segment) {
        state
            .ticket(&session_id, &ticket_id)
            .map(|ticket| json(&ticket))
    } else {
        return not_found();
    };
    match result {
        Ok(response) => response,
        Err(TicketError::UnknownSession) => json_status(404, &json!({"error": "unknown session"})),
        Err(TicketError::UnknownTicket) => json_status(404, &json!({"error": "unknown ticket"})),
        Err(TicketError::InvalidPattern | TicketError::Io(_)) => {
            json_status(500, &json!({"error": "could not read tickets"}))
        }
    }
}

/// `POST /sessions/<session id>/tickets/<ticket id>/implement` (ticket 20).
pub(crate) fn implement_response(state: &State, path: &str) -> Response<io::Cursor<Vec<u8>>> {
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let ["sessions", session, "tickets", ticket, "implement"] = parts[..] else {
        return Response::from_string("not found").with_status_code(404);
    };
    let (Some(session), Some(ticket)) = (decode_segment(session), decode_segment(ticket)) else {
        return Response::from_string("not found").with_status_code(404);
    };
    match state.implement_ticket(&session, &ticket) {
        Ok(text) => json(&json!({ "pasted": text })),
        Err(ImplementError::Blocked) => json_status(
            409,
            &json!({"error": "blocked by a ticket that is still pending"}),
        ),
        Err(ImplementError::NotPending) => {
            json_status(409, &json!({"error": "this ticket is already done"}))
        }
        Err(ImplementError::WindowNotFound) => json_status(
            409, &json!({"error": "session window was not found"}),
        ),
        Err(ImplementError::Ticket(TicketError::UnknownSession)) => {
            json_status(404, &json!({"error": "unknown session"}))
        }
        Err(ImplementError::Ticket(TicketError::UnknownTicket)) => {
            json_status(404, &json!({"error": "unknown ticket"}))
        }
        Err(ImplementError::Ticket(_)) => {
            json_status(500, &json!({"error": "could not read tickets"}))
        }
    }
}

fn decode_segment(segment: &str) -> Option<std::borrow::Cow<'_, str>> {
    if segment.is_empty()
        || segment.contains('/')
        || segment.split('%').skip(1).any(|value| {
            !value
                .as_bytes()
                .get(..2)
                .is_some_and(|bytes| bytes.iter().all(u8::is_ascii_hexdigit))
        })
    {
        return None;
    }
    percent_decode_str(segment).decode_utf8().ok()
}

pub(crate) fn json_status<T: serde::Serialize>(
    status: u16,
    value: &T,
) -> Response<io::Cursor<Vec<u8>>> {
    json(value).with_status_code(status)
}

pub(crate) fn json<T: serde::Serialize>(value: &T) -> Response<io::Cursor<Vec<u8>>> {
    let body = serde_json::to_vec(value).expect("serializable");
    let header = Header::from_bytes("Content-Type", "application/json").expect("valid header");
    Response::from_data(body).with_header(header)
}
