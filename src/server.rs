use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::fd::{AsRawFd, FromRawFd};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use libc::{self, EVFILT_READ, EV_ADD, EV_ENABLE, EV_EOF};

use crate::config::{ServerConfig, SingleServerConfig};
use crate::io::kqueue;
use crate::session::SessionManager;
use uuid::Uuid;

const CONNECTION_TIMEOUT_SECS: u64 = 2;
const MAX_CONNECTIONS: usize = 100;

pub fn run(all_configs: Vec<ServerConfig>) -> std::io::Result<()> {
    // --- Setup listeners ---
    let mut configs_by_port = HashMap::<u16, Vec<SingleServerConfig>>::new();
    for config in &all_configs {
        for server_config in &config.servers {
            for port in &server_config.ports {
                configs_by_port
                    .entry(*port)
                    .or_default()
                    .push(server_config.clone());
            }
        }
    }

    for (_, configs) in &configs_by_port {
        let mut names = HashSet::new();
        for config in configs {
            if !names.insert(&config.server_name) {
                eprintln!("Warning: Duplicate server_name '{}' found for the same port. The first defined server will be used.", config.server_name);
            }
        }
    }

    let mut listeners = Vec::new();
    let mut server_configs_by_fd = HashMap::<i32, Vec<SingleServerConfig>>::new();

    for (port, configs) in configs_by_port {
        if configs.is_empty() {
            continue;
        }
        let addr = format!("{}:{}", configs[0].host, port);
        let listener = TcpListener::bind(&addr)?;
        listener.set_nonblocking(true)?;
        println!("Server listening on {}", addr);
        let lfd = listener.as_raw_fd();
        server_configs_by_fd.insert(lfd, configs);
        listeners.push(listener);
    }

    // --- Create kqueue ---
    let kq = kqueue::kqueue()?;
    println!("Server initialized, waiting for events...");

    // --- Session Manager ---
    let session_manager = Arc::new(Mutex::new(SessionManager::new(3600))); // 1 hour timeout

    // --- Connection activity tracking ---
    let mut connections_activity: HashMap<i32, Instant> = HashMap::new();
    let mut client_server_configs: HashMap<i32, Vec<SingleServerConfig>> = HashMap::new();
    let mut connection_buffers: HashMap<i32, Vec<u8>> = HashMap::new();

    // --- Register listener fds for read events ---
    let changes: Vec<libc::kevent> = listeners
        .iter()
        .map(|listener| libc::kevent {
            ident: listener.as_raw_fd() as libc::uintptr_t,
            filter: EVFILT_READ,
            flags: EV_ADD | EV_ENABLE,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        })
        .collect();

    kqueue::kevent(kq, &changes, &mut [], None)?;

    // --- Event loop ---
    loop {
        match session_manager.lock() {
            Ok(mut guard) => guard.clean_expired_sessions(),
            Err(poisoned) => {
                eprintln!("Session manager lock was poisoned: {}. Recovering...", poisoned);
                let mut guard = poisoned.into_inner();
                guard.clean_expired_sessions();
            }
        }
        
        // --- Check for connection timeouts ---
        let now = Instant::now();
        connections_activity.retain(|&fd, last_activity| {
            if now.duration_since(*last_activity).as_secs() > CONNECTION_TIMEOUT_SECS {
                println!("Client fd {} timed out, closing", fd);
                unsafe {
                    libc::close(fd);
                    client_server_configs.remove(&fd);
                    connection_buffers.remove(&fd);
                }
                false // Remove from map
            } else {
                true // Keep in map
            }
        });

        let mut events: [libc::kevent; 16] = unsafe { std::mem::zeroed() };

        let nev = kqueue::kevent(kq, &[], &mut events, Some(Duration::from_millis(100)))?;
        if nev == 0 {
            continue; // timeout — loop again
        }

        for ev in &events[..nev as usize] {
            let fd = ev.ident as i32;

            if let Some(server_configs) = server_configs_by_fd.get(&fd) {
                // --- New connection ---
                let listener = listeners.iter().find(|l| l.as_raw_fd() == fd).unwrap();
                if let Ok((stream, addr)) = listener.accept() {
                    if connections_activity.len() >= MAX_CONNECTIONS {
                        println!("Max connections reached, rejecting new connection from {}", addr);
                        drop(stream); // Close the connection
                        continue;
                    }

                    println!("Accepted connection from {}", addr);
                    stream.set_nonblocking(true)?;
                    let cfd = stream.as_raw_fd();
                    println!("Registered client {}", cfd);
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
                    client_server_configs.insert(cfd, server_configs.clone());
                    connection_buffers.insert(cfd, Vec::new());

                    // Leak the stream so it stays open; we’ll recreate from fd on read
                    std::mem::forget(stream);
                }
            } else if ev.filter == EVFILT_READ {
                // --- Data available from client ---
                if let Some(server_configs) = client_server_configs.get(&fd) {
                    let buffer = connection_buffers.entry(fd).or_insert_with(Vec::new);
                    let mut stream_owner = unsafe { std::net::TcpStream::from_raw_fd(fd) };

                    let mut chunk = [0u8; 4096];
                    match stream_owner.read(&mut chunk) {
                        Ok(0) => {
                            println!("Client fd {} closed connection", fd);
                            client_server_configs.remove(&fd);
                            connection_buffers.remove(&fd);
                        }
                        Ok(n) => {
                            buffer.extend_from_slice(&chunk[..n]);

                            match crate::http::request::parse_request_from_buffer(buffer) {
                                Ok(Some((request, consumed))) => {
                                    connections_activity.insert(fd, Instant::now());
                                    let host = request.headers.get("host").map(|s| s.as_str()).unwrap_or("");
                                    let host_without_port = host.split(':').next().unwrap_or("");
                                    let server_config = server_configs
                                        .iter()
                                        .find(|c| c.server_name == host_without_port)
                                        .unwrap_or_else(|| &server_configs[0]);

                                    if request.body.len() > server_config.client_max_body_size {
                                        let response = crate::http::response::Response::new(
                                            413,
                                            b"Payload Too Large".to_vec(),
                                        );
                                        let _ = stream_owner.write_all(&response.to_bytes());
                                    } else {
                                        let response = {
                                            let mut session_manager_lock = match session_manager.lock() {
                                                Ok(guard) => guard,
                                                Err(poisoned) => poisoned.into_inner(),
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

                                            match crate::handler::find_route(&request, server_config) {
                                                Some(route) => {
                                                    let mut resp = crate::handler::handle_request(
                                                        &request,
                                                        route,
                                                        server_config,
                                                        &mut current_session,
                                                    );
                                                    if let Some(sid) = session_id {
                                                        if !request.cookies.contains_key("session_id") {
                                                            resp.headers.insert(
                                                                "Set-Cookie".to_string(),
                                                                format!(
                                                                    "session_id={}; HttpOnly; Path=/",
                                                                    sid
                                                                ),
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
                                        let _ = stream_owner.write_all(&response.to_bytes());
                                    }

                                    buffer.drain(..consumed);
                                }
                                Ok(None) => {
                                    std::mem::forget(stream_owner);
                                }
                                Err(_) => {
                                    let response = crate::http::response::Response::new(400, b"Bad Request".to_vec());
                                    let _ = stream_owner.write_all(&response.to_bytes());
                                    client_server_configs.remove(&fd);
                                    connection_buffers.remove(&fd);
                                }
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::mem::forget(stream_owner);
                        }
                        Err(e) => {
                            eprintln!("Read error on fd {}: {}", fd, e);
                            client_server_configs.remove(&fd);
                            connection_buffers.remove(&fd);
                        }
                    }
                }
            } else if ev.flags & EV_EOF != 0 {
                unsafe {
                    libc::close(fd);
                    client_server_configs.remove(&fd);
                    connection_buffers.remove(&fd);
                }
            }
        }
    }
}