//! The on-demand HTTP server for Phones.

use std::io;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use qrcode::{render::svg, QrCode};
use serde::Serialize;
use tiny_http::{Header, Method, Request, Response, Server};

use crate::{server, Core, State};

const IDLE_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const IDLE_POLL_INTERVAL: Duration = Duration::from_secs(1);
const TOKEN_COOKIE: &str = "skills_pad_token";

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
    address_override: Option<IpAddr>,
    running: Option<Running>,
}

impl Sharing {
    fn address(&self) -> io::Result<IpAddr> {
        let address = match self.address_override {
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
    pub fn set_sharing_address_override(&self, address: Option<IpAddr>) {
        self.sharing.lock().unwrap().address_override = address;
    }

    /// Return the address the next start will use, without starting a listener.
    pub fn sharing_address(&self) -> io::Result<IpAddr> {
        self.sharing.lock().unwrap().address()
    }

    /// Start sharing on a fresh port with a new token, replacing any active sharing.
    pub fn start_sharing(&self) -> io::Result<SharingInfo> {
        let mut sharing = self.sharing.lock().unwrap();
        let running = Running::start(self.state.clone(), sharing.address()?)?;
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
            .name("skills-pad-sharing".into())
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

fn handle(state: &State, request: Request, token: &str) -> bool {
    let (path, query) = request.url().split_once('?').unwrap_or((request.url(), ""));
    let from_url = query
        .split('&')
        .any(|param| param.strip_prefix("token=") == Some(token));
    let from_cookie = request
        .headers()
        .iter()
        .filter(|header| header.field.equiv("Cookie"))
        .flat_map(|header| header.value.as_str().split(';'))
        .any(|cookie| cookie.trim().split_once('=') == Some((TOKEN_COOKIE, token)));

    let authenticated = from_url || from_cookie;
    let mut response = if authenticated {
        match (request.method(), path) {
            (Method::Get, "/") => Response::from_string(
                "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\">\
                 <title>Skills Pad</title><h1>Skills Pad</h1></html>",
            )
            .with_header(header("Content-Type", "text/html; charset=utf-8"))
            .boxed(),
            (Method::Get, "/skills") => server::skills_response(state).boxed(),
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
    if from_url {
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

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name, value).expect("valid header")
}
