//! The Core's local HTTP API.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread::JoinHandle;

use tiny_http::{Header, Method, Request, Response, Server};

use crate::{skills, State};

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

fn handle(state: &State, request: Request) {
    let path = request.url().split('?').next().unwrap_or("").to_string();
    let response = match (request.method(), path.as_str()) {
        (Method::Get, "/skills") => json(&skills::scan(&state.home)),
        (Method::Get, "/harnesses") => json(&skills::harnesses(&skills::scan(&state.home))),
        _ => Response::from_string("not found").with_status_code(404),
    };
    let _ = request.respond(response);
}

fn json<T: serde::Serialize>(value: &T) -> Response<io::Cursor<Vec<u8>>> {
    let body = serde_json::to_vec(value).expect("serializable");
    let header = Header::from_bytes("Content-Type", "application/json").expect("valid header");
    Response::from_data(body).with_header(header)
}
