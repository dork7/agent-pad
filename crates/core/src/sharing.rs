//! The on-demand HTTP server for Phones.

use std::io::{self, Read};
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use qrcode::{render::svg, QrCode};
use serde::Serialize;
use tiny_http::{Header, Method, Request, Response, Server};

use crate::phones::Admission;
use crate::{server, Core, PasteError, State};

const IDLE_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const IDLE_POLL_INTERVAL: Duration = Duration::from_secs(1);
// Cookie names keep their pre-rename "skills_pad" spelling on purpose, so phones paired
// before the rename to Agent Pad stay paired.
const TOKEN_COOKIE: &str = "skills_pad_token";
const DEVICE_COOKIE: &str = "skills_pad_device";
/// The device ID cookie lasts a year so a Phone keeps its name and View.
const DEVICE_COOKIE_MAX_AGE: u32 = 365 * 24 * 60 * 60;

/// The phone page, embedded in the binary.
const PHONE_PAGE: &str = include_str!("phone.html");

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SharingInfo {
    /// The URL a Phone opens to pair, including the current sharing token.
    pub url: String,
    pub address: IpAddr,
    pub port: u16,
    pub qr_svg: String,
}

#[derive(Default)]
pub(crate) struct Sharing {
    running: Option<Running>,
}

impl Sharing {
    fn address(address_override: Option<IpAddr>) -> io::Result<IpAddr> {
        let address = match address_override {
            Some(address) => address,
            None => primary_lan_address()?,
        };
        if address.is_unspecified() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "sharing requires a specific network address",
            ));
        }
        Ok(address)
    }
}

impl Core {
    /// Choose the address for the next start. Pass `None` to use the primary LAN address.
    /// The choice is saved in settings.
    pub fn set_sharing_address_override(&self, address: Option<IpAddr>) {
        let saved = self.state.change_settings(|s| {
            s.network_override = address;
            Ok(())
        });
        if let Err(error) = saved {
            eprintln!("agent-pad: {error}");
        }
    }

    /// The saved address override, if any.
    pub fn sharing_address_override(&self) -> Option<IpAddr> {
        self.state.settings.lock().unwrap().network_override
    }

    /// Return the address the next start will use, without starting a listener.
    pub fn sharing_address(&self) -> io::Result<IpAddr> {
        Sharing::address(self.sharing_address_override())
    }

    /// Start sharing on a fresh port with a new token, replacing any active sharing.
    pub fn start_sharing(&self) -> io::Result<SharingInfo> {
        let mut sharing = self.sharing.lock().unwrap();
        let running = Running::start(
            self.state.clone(),
            Sharing::address(self.sharing_address_override())?,
        )?;
        let info = running.info.clone();
        sharing.running = Some(running);
        Ok(info)
    }

    /// Stop serving Phones and discard the current token.
    pub fn stop_sharing(&self) {
        let mut sharing = self.sharing.lock().unwrap();
        drop(sharing.running.take());
    }

    /// Return the active sharing details, or `None` when sharing is off.
    pub fn sharing_status(&self) -> Option<SharingInfo> {
        let mut sharing = self.sharing.lock().unwrap();
        if sharing.running.as_ref().is_some_and(Running::is_finished) {
            sharing.running = None;
        }
        sharing.running.as_ref().map(|running| running.info.clone())
    }
}

fn primary_lan_address() -> io::Result<IpAddr> {
    // Connecting a UDP socket selects the outbound interface without sending packets.
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("192.0.2.1:80")?;
    Ok(socket.local_addr()?.ip())
}

struct Running {
    info: SharingInfo,
    // The worker owns the server so that reaching the idle deadline closes the listener
    // even when no Pad command is called.
    server: Weak<Server>,
    stopping: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Running {
    fn start(state: Arc<State>, address: IpAddr) -> io::Result<Self> {
        let mut bytes = [0; 16];
        getrandom::fill(&mut bytes).map_err(io::Error::other)?;
        let token = format!("{:032x}", u128::from_be_bytes(bytes));

        let server = Arc::new(Server::http(SocketAddr::new(address, 0)).map_err(io::Error::other)?);
        let bound = server
            .server_addr()
            .to_ip()
            .ok_or_else(|| io::Error::other("server is not bound to an IP address"))?;
        let url = format!("http://{bound}/?token={token}");
        let qr_svg = QrCode::new(&url)
            .map_err(io::Error::other)?
            .render::<svg::Color>()
            .min_dimensions(192, 192)
            .build();
        let info = SharingInfo {
            url,
            address: bound.ip(),
            port: bound.port(),
            qr_svg,
        };

        let server_ref = Arc::downgrade(&server);
        let stopping = Arc::new(AtomicBool::new(false));
        let worker_stopping = stopping.clone();
        let started_at = state.clock.now();
        let thread = thread::Builder::new()
            .name("agent-pad-sharing".into())
            .spawn(move || serve(&state, &server, &worker_stopping, &token, started_at))?;
        Ok(Self {
            info,
            server: server_ref,
            stopping,
            thread: Some(thread),
        })
    }

    fn is_finished(&self) -> bool {
        self.thread.as_ref().is_none_or(JoinHandle::is_finished)
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Relaxed);
        if let Some(server) = self.server.upgrade() {
            server.unblock();
        }
        // Join before another Pad command can observe the stopped state.
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(
    state: &State,
    server: &Server,
    stopping: &AtomicBool,
    token: &str,
    mut last_activity: Instant,
) {
    while !stopping.load(Ordering::Relaxed) {
        let Ok(request) = server.recv_timeout(IDLE_POLL_INTERVAL) else {
            break;
        };
        let now = state.clock.now();
        if stopping.load(Ordering::Relaxed) || now.duration_since(last_activity) >= IDLE_TIMEOUT {
            break;
        }
        if let Some(request) = request {
            if handle(state, request, token) {
                last_activity = now;
            }
        }
    }
}

fn handle(state: &State, mut request: Request, token: &str) -> bool {
    let url = request.url().to_string();
    let (path, query) = url.split_once('?').unwrap_or((&url, ""));
    let from_url = query
        .split('&')
        .any(|param| param.strip_prefix("token=") == Some(token));
    let cookie = |name: &str| {
        request
            .headers()
            .iter()
            .filter(|header| header.field.equiv("Cookie"))
            .flat_map(|header| header.value.as_str().split(';'))
            .filter_map(|c| c.trim().split_once('='))
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v.to_string())
    };
    let from_cookie = cookie(TOKEN_COOKIE).as_deref() == Some(token);
    let device_cookie = cookie(DEVICE_COOKIE);
    let user_agent = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("User-Agent"))
        .map(|h| h.value.as_str().to_string())
        .unwrap_or_default();

    let admission = (from_url || from_cookie)
        .then(|| state.admit_phone(device_cookie.as_deref(), from_url, &user_agent));
    let (device, new_device) = match &admission {
        Some(Admission::Admitted {
            device_id,
            new_cookie,
        }) => (
            Some(device_id.clone()),
            new_cookie.then(|| device_id.clone()),
        ),
        _ => (None, None),
    };
    let authenticated = device.is_some();
    let mut response = if let Some(device) = &device {
        match (request.method(), path) {
            (Method::Get, "/") => Response::from_string(PHONE_PAGE)
                .with_header(header("Content-Type", "text/html; charset=utf-8"))
                .boxed(),
            (Method::Get, "/skills") => server::json(&state.skills_for_phone(device)).boxed(),
            (Method::Get, "/harnesses") => server::json(&state.harnesses_for_phone(device)).boxed(),
            (Method::Get, "/view") => server::json(&state.view_for_phone(device)).boxed(),
            (Method::Get, "/snippets") => server::json(&state.snippets_for_phone(device)).boxed(),
            (Method::Post, "/paste-snippet") => paste_snippet(state, device, &mut request).boxed(),
            (Method::Get, "/sessions") => server::json(&state.sessions_for_phone(device)).boxed(),
            (Method::Get, "/hook-status") => server::json(&serde_json::json!({
                "installed": state.hooks_installed()
            })).boxed(),
            (Method::Get, path) if path.starts_with("/sessions/") => {
                if sees_session(state, device, path) {
                    server::tickets_response(state, path).boxed()
                } else {
                    outside_view().boxed()
                }
            }
            (Method::Post, path) if path.starts_with("/sessions/") => {
                if sees_session(state, device, path) {
                    if path.ends_with("/run") {
                        server::run_response(state, path, &mut request, Some(device)).boxed()
                    } else {
                        server::implement_response(state, path).boxed()
                    }
                } else {
                    outside_view().boxed()
                }
            }
            (Method::Post, "/paste") => paste(state, device, &mut request).boxed(),
            (Method::Post, "/heartbeat") => Response::empty(204).boxed(),
            _ => Response::from_string("not found")
                .with_status_code(404)
                .boxed(),
        }
    } else {
        Response::from_string("unauthorized")
            .with_status_code(401)
            .boxed()
    };
    if let Some(device_id) = new_device {
        response.add_header(header(
            "Set-Cookie",
            &format!(
                "{DEVICE_COOKIE}={device_id}; Path=/; Max-Age={DEVICE_COOKIE_MAX_AGE}; HttpOnly; SameSite=Lax"
            ),
        ));
    }
    if from_url && authenticated {
        response.add_header(header(
            "Set-Cookie",
            &format!("{TOKEN_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict"),
        ));
    }
    response.add_header(header("Cache-Control", "no-store"));
    response.add_header(header("Referrer-Policy", "no-referrer"));
    let _ = request.respond(response);
    authenticated
}

/// `POST /paste` with `{"id": "<skill id>"}`. Only an ID is accepted; the Core builds the text.
fn paste(state: &State, device: &str, request: &mut Request) -> Response<io::Cursor<Vec<u8>>> {
    #[derive(serde::Deserialize)]
    struct Body {
        id: String,
        #[serde(default)]
        preset: Option<String>,
    }
    let mut body = String::new();
    let parsed = request
        .as_reader()
        .take(64 * 1024)
        .read_to_string(&mut body)
        .ok()
        .and_then(|_| serde_json::from_str::<Body>(&body).ok());
    let Some(Body { id, preset }) = parsed else {
        return server::json_status(
            400,
            &serde_json::json!({ "error": "expected {\"id\": ...}" }),
        );
    };
    let result = match &preset {
        Some(preset) => state.paste_preset_for_phone(device, &id, preset),
        None => state.paste_for_phone(device, &id).map(|_| ()),
    };
    paste_result(result, &id)
}

fn paste_result(result: Result<(), PasteError>, id: &str) -> Response<io::Cursor<Vec<u8>>> {
    match result {
        Ok(()) => server::json_status(200, &serde_json::json!({ "pasted": id })),
        Err(PasteError::UnknownSkill) => {
            server::json_status(404, &serde_json::json!({ "error": "unknown skill" }))
        }
        Err(PasteError::NotInView) => server::json_status(
            403,
            &serde_json::json!({ "error": "not in this phone's view" }),
        ),
        Err(PasteError::UnknownPreset) => {
            server::json_status(404, &serde_json::json!({ "error": "unknown preset" }))
        }
        Err(PasteError::UnknownSnippet) => {
            server::json_status(404, &serde_json::json!({ "error": "unknown snippet" }))
        }
    }
}

/// Whether the Session named in `/sessions/<id>/...` is in this Phone's View. Unknown
/// sessions pass through so the route answers 404 for them.
fn sees_session(state: &State, device: &str, path: &str) -> bool {
    let Some(raw) = path
        .strip_prefix("/sessions/")
        .and_then(|rest| rest.split('/').next())
    else {
        return false;
    };
    let id = percent_encoding::percent_decode_str(raw).decode_utf8_lossy();
    let known = state.list_sessions().iter().any(|s| s.session_id == id);
    !known
        || state
            .sessions_for_phone(device)
            .iter()
            .any(|s| s.session_id == id)
}

fn outside_view() -> Response<io::Cursor<Vec<u8>>> {
    server::json_status(
        403,
        &serde_json::json!({ "error": "not in this phone's view" }),
    )
}

/// `POST /paste-snippet` with `{"id": "<snippet id>"}`.
fn paste_snippet(
    state: &State,
    device: &str,
    request: &mut Request,
) -> Response<io::Cursor<Vec<u8>>> {
    #[derive(serde::Deserialize)]
    struct Body {
        id: String,
    }
    let mut body = String::new();
    let parsed = request
        .as_reader()
        .take(64 * 1024)
        .read_to_string(&mut body)
        .ok()
        .and_then(|_| serde_json::from_str::<Body>(&body).ok());
    let Some(Body { id }) = parsed else {
        return server::json_status(
            400,
            &serde_json::json!({ "error": "expected {\"id\": ...}" }),
        );
    };
    paste_result(state.paste_snippet_for_phone(device, &id), &id)
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name, value).expect("valid header")
}
