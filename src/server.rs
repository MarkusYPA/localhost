use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::fd::{AsRawFd, FromRawFd};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use libc::{self, EVFILT_READ, EV_ADD, EV_ENABLE, EV_EOF};

use crate::config::ServerConfig;
use crate::io::kqueue;
use crate::session::SessionManager;
use uuid::Uuid;

const CONNECTION_TIMEOUT_SECS: u64 = 30;
const MAX_CONNECTIONS: usize = 100;

pub fn run(config: ServerConfig) -> std::io::Result<()> {
    // --- Setup listener ---
    let addr = format!("{}:{}", config.host, config.ports[0]);
    let listener = TcpListener::bind(&addr)?;
    listener.set_nonblocking(true)?;
    println!("Server listening on {addr}");

    // --- Create kqueue ---
    let kq = kqueue::kqueue()?;
    println!("Server initialized, waiting for events...");

    // --- Session Manager ---
    let session_manager = Arc::new(Mutex::new(SessionManager::new(3600))); // 1 hour timeout

    // --- Connection activity tracking ---
    let mut connections_activity: HashMap<i32, Instant> = HashMap::new();

    // --- Register listener fd for read events ---
    let lfd = listener.as_raw_fd();
    let change = libc::kevent {
        ident: lfd as libc::uintptr_t,
        filter: EVFILT_READ,
        flags: EV_ADD | EV_ENABLE,
        fflags: 0,
        data: 0,
        udata: std::ptr::null_mut(),
    };

    kqueue::kevent(kq, std::slice::from_ref(&change), &mut [], None)?;

    // --- Event loop ---
    loop {
        match session_manager.lock() {
            Ok(mut guard) => guard.clean_expired_sessions(),
            Err(poisoned) => {
                eprintln!("Session manager lock was poisoned: {poisoned}. Recovering...");
                let mut guard = poisoned.into_inner();
                guard.clean_expired_sessions();
            }
        }

        let mut events: [libc::kevent; 16] = unsafe { std::mem::zeroed() };

        let nev = kqueue::kevent(kq, &[], &mut events, Some(Duration::from_secs(5)))?;
        if nev == 0 {
            continue; // timeout — loop again
        }

        for ev in &events[..nev as usize] {
            let fd = ev.ident as i32;

            if fd == lfd {
                // --- New connection ---
                if let Ok((stream, _addr)) = listener.accept() {
                    if connections_activity.len() >= MAX_CONNECTIONS {
                        println!("Max connections reached, rejecting new connection from {addr}");
                        drop(stream); // Close the connection
                        continue;
                    }

                    println!("Accepted connection from {addr}");
                    stream.set_nonblocking(true)?;
                    let cfd = stream.as_raw_fd();

                    // Register client fd for read events
                    let change = libc::kevent {
                        ident: cfd as libc::uintptr_t,
                        filter: EVFILT_READ,
                        flags: EV_ADD | EV_ENABLE,
                        fflags: 0,
                        data: 0,
                        udata: std::ptr::null_mut(),
                    };
                    kqueue::kevent(kq, std::slice::from_ref(&change), &mut [], None)?;

                    // Add to activity tracking
                    connections_activity.insert(cfd, Instant::now());

                    // Leak the stream so it stays open; we’ll recreate from fd on read
                    std::mem::forget(stream);

                }
            } else if ev.filter == EVFILT_READ {
                // --- Data available from client ---
                unsafe {
                    let mut stream = std::net::TcpStream::from_raw_fd(fd);
                    let mut buf = [0u8; 1024];
                    match stream.read(&mut buf) {
                        Ok(0) => {
                            // client closed, just drop stream
                            println!("Client fd {fd} closed connection");
                        }
                        Ok(n) => {
                            connections_activity.insert(fd, Instant::now()); // Update activity time
                            let request = crate::http::request::Request::from(&buf[..n]);


                            let response = {
                                let mut session_manager_lock = match session_manager.lock() {
                                    Ok(guard) => guard,
                                    Err(poisoned) => {
                                        eprintln!("Session manager lock was poisoned: {poisoned}. Recovering...");
                                        poisoned.into_inner()
                                    }
                                };
                                let mut session_id: Option<Uuid> = None;
                                if let Some(cookie_header) = request.cookies.get("session_id") {
                                    if let Ok(uuid) = Uuid::parse_str(cookie_header) {
                                        session_id = Some(uuid);
                                    }
                                }

                                let mut current_session = if let Some(id) = session_id {
                                    session_manager_lock.get_session(&id)
                                } else {
                                    None
                                };

                                if current_session.is_none() {
                                    println!("No valid session found, creating a new one.");
                                    let new_session = session_manager_lock.create_session();
                                    session_id = Some(new_session.id);
                                    current_session =
                                        session_manager_lock.get_session(&new_session.id);
                                }

                                match crate::handler::find_route(&request, &config) {
                                    Some(route) => {
                                        let mut resp = crate::handler::handle_request(
                                            &request,
                                            route,
                                            &config,
                                            &mut current_session,
                                        );
                                        if let Some(sid) = session_id {
                                            if !request.cookies.contains_key("session_id") {
                                                resp.headers.insert(
                                                    "Set-Cookie".to_string(),
                                                    format!("session_id={sid}; HttpOnly; Path=/"),
                                                );
                                            }
                                        }
                                        resp
                                    }
                                    None => crate::http::response::Response::new(
                                        404,
                                        b"Not Found".to_vec(),
                                    ),
                                }
                            };

                            let _ = stream.write_all(&response.to_bytes());
                            // don't call libc::close(fd); Rust will close when stream drops
                        }
                        Err(e) => {
                            eprintln!("Read error on fd {fd}: {e}");   
                            // don't call libc::close(fd)
                        }
                    } // stream dropped here, fd automatically closed
                }
            } else if ev.flags & EV_EOF != 0 {

                unsafe {
                    libc::close(fd);
                }
            }
        }

        // --- Check for connection timeouts ---
        let now = Instant::now();
        connections_activity.retain(|&fd, last_activity| {
            if now.duration_since(*last_activity).as_secs() > CONNECTION_TIMEOUT_SECS {
                println!("Client fd {fd} timed out, closing");
                unsafe {
                    libc::close(fd);
                }
                false // Remove from map
            } else {
                true // Keep in map
            }
        });
    }
}
