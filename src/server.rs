use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::fd::{AsRawFd, FromRawFd};
use std::time::Duration;
use std::sync::{Arc, Mutex};

use libc::{self, EV_ADD, EV_ENABLE, EV_EOF, EVFILT_READ};

use crate::config::ServerConfig;
use crate::io::kqueue;
use crate::session::SessionManager;
use uuid::Uuid;

pub fn run(config: ServerConfig) -> std::io::Result<()> {
    // --- Setup listener ---
    let addr = format!("{}:{}", config.host, config.ports[0]);
    let listener = TcpListener::bind(&addr)?;
    listener.set_nonblocking(true)?;
    println!("Server listening on {}", addr);

    // --- Create kqueue ---
    let kq = kqueue::kqueue()?;
    println!("Server initialized, waiting for events...");

    // --- Session Manager ---
    let session_manager = Arc::new(Mutex::new(SessionManager::new(3600))); // 1 hour timeout

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
        session_manager.lock().unwrap().clean_expired_sessions();

        let mut events: [libc::kevent; 16] = unsafe { std::mem::zeroed() };

        let nev = kqueue::kevent(kq, &[], &mut events, Some(Duration::from_secs(5)))?;
        if nev == 0 {
            continue; // timeout — loop again
        }

        for ev in &events[..nev as usize] {
            let fd = ev.ident as i32;

            if fd == lfd {
                // --- New connection ---
                if let Ok((stream, addr)) = listener.accept() {
                    println!("Accepted connection from {}", addr);
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

                    // Leak the stream so it stays open; we’ll recreate from fd on read
                    std::mem::forget(stream);
                    println!("Registered client fd {} for read events", cfd);
                }
            } else if ev.filter == EVFILT_READ {
                // --- Data available from client ---
                unsafe {
                    let mut stream = std::net::TcpStream::from_raw_fd(fd);
                    let mut buf = [0u8; 1024];
                    match stream.read(&mut buf) {
                        Ok(0) => {
                            // client closed, just drop stream
                            println!("Client fd {} closed connection", fd);
                        }
                        Ok(n) => {
                            let request = crate::http::request::Request::from(&buf[..n]);
                            println!("{:?}", request);

                            let response = {
                                let mut session_manager_lock = session_manager.lock().unwrap();
                                let mut session_id: Option<Uuid> = None;
                                if let Some(cookie_header) = request.cookies.get("session_id") {
                                    if let Ok(uuid) = Uuid::parse_str(cookie_header) {
                                        session_id = Some(uuid);
                                    }
                                }

                                let mut current_session = if let Some(id) = session_id {
                                    println!("Attempting to retrieve session: {}", id);
                                    session_manager_lock.get_session(&id)
                                } else {
                                    None
                                };

                                if current_session.is_none() {
                                    println!("No valid session found, creating a new one.");
                                    let new_session = session_manager_lock.create_session();
                                    session_id = Some(new_session.id);
                                    current_session = session_manager_lock.get_session(&new_session.id);
                                }

                                match crate::handler::find_route(&request, &config) {
                                    Some(route) => {
                                        let mut resp = crate::handler::handle_request(&request, route, &mut current_session);
                                        if let Some(sid) = session_id {
                                            if request.cookies.get("session_id").is_none() {
                                                resp.headers.insert("Set-Cookie".to_string(), format!("session_id={}; HttpOnly; Path=/", sid));
                                            }
                                        }
                                        resp
                                    },
                                    None => crate::http::response::Response::new(404, b"Not Found".to_vec()),
                                }
                            };

                            let _ = stream.write_all(&response.to_bytes());
                            // don't call libc::close(fd); Rust will close when stream drops
                        }
                        Err(e) => {
                            eprintln!("Read error on fd {}: {}", fd, e);
                            // don't call libc::close(fd)
                        }
                    } // stream dropped here, fd automatically closed
                }
            } else if ev.flags & EV_EOF != 0 {
                println!("EOF on fd {}, closing", fd);
                unsafe {
                    libc::close(fd);
                }
            }
        }
    }
}
