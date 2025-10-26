use crate::config::{ServerConfig, SingleServerConfig};
use crate::io::kqueue;
use crate::session::SessionManager;
use libc::{self, EVFILT_READ, EVFILT_WRITE, EV_ADD, EV_ENABLE, EV_EOF};
use log::{debug, error, info, warn};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::os::fd::{AsRawFd, FromRawFd};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

const CONNECTION_TIMEOUT_SECS: u64 = 2;
const MAX_CONNECTIONS: usize = 1000;

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
                warn!(
                    "Duplicate server_name '{}' found for the same port. The first defined server will be used.",
                    config.server_name
                );
            }
        }
    }

    let mut listeners = Vec::new();
    let mut server_configs_by_fd = HashMap::<i32, Vec<SingleServerConfig>>::new();

    for (port, configs) in configs_by_port {
        if configs.is_empty() {
            continue;
        }
        let addr = format!("{}:{port}", configs[0].host);
        let listener = TcpListener::bind(&addr)?;
        listener.set_nonblocking(true)?;
        info!("Server listening on {addr}");
        let lfd = listener.as_raw_fd();
        server_configs_by_fd.insert(lfd, configs);
        listeners.push(listener);
    }

    // --- Create kqueue ---
    let kq = kqueue::kqueue()?;
    info!("Server initialized, waiting for events...");

    // --- Session Manager ---
    let session_manager = Arc::new(Mutex::new(SessionManager::new(3600))); // 1 hour timeout

    // --- Connection activity tracking ---
    let mut connections_activity: HashMap<i32, Instant> = HashMap::new();
    let mut client_server_configs: HashMap<i32, Vec<SingleServerConfig>> = HashMap::new();
    let mut connection_buffers: HashMap<i32, Vec<u8>> = HashMap::new();
    let mut client_addresses: HashMap<i32, SocketAddr> = HashMap::new();
    let mut server_addresses: HashMap<i32, SocketAddr> = HashMap::new();
    let mut write_buffers: HashMap<i32, (usize, Vec<u8>)> = HashMap::new();

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
                error!(
                    "Session manager lock was poisoned: {}. Recovering...",
                    poisoned
                );
                let mut guard = poisoned.into_inner();
                guard.clean_expired_sessions();
            }
        }

        // --- Check for connection timeouts ---
        let now = Instant::now();
        connections_activity.retain(|&fd, last_activity| {
            if now.duration_since(*last_activity).as_secs() > CONNECTION_TIMEOUT_SECS {
                info!("Client fd {fd} timed out, closing");
                unsafe {
                    libc::close(fd);
                    client_server_configs.remove(&fd);
                    connection_buffers.remove(&fd);
                    client_addresses.remove(&fd);
                    server_addresses.remove(&fd);
                    write_buffers.remove(&fd);
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
                        warn!("Max connections reached, rejecting new connection from {addr}");
                        drop(stream); // Close the connection
                        continue;
                    }

                    info!("Accepted connection from {addr}");
                    let local_addr = stream.local_addr()?;
                    stream.set_nonblocking(true)?;
                    let cfd = stream.as_raw_fd();
                    debug!("Registered client {cfd}");
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
                    client_addresses.insert(cfd, addr);
                    server_addresses.insert(cfd, local_addr);

                    // Leak the stream so it stays open; we’ll recreate from fd on read
                    std::mem::forget(stream);
                }
            } else if ev.filter == EVFILT_WRITE {
                // --- Socket ready for writing ---
                if let Some((offset, buffer)) = write_buffers.get_mut(&fd) {
                    let mut stream = unsafe { std::net::TcpStream::from_raw_fd(fd) };
                    match stream.write(&buffer[*offset..]) {
                        Ok(n) => {
                            *offset += n;
                            if *offset == buffer.len() {
                                // All data written, remove from buffer
                                write_buffers.remove(&fd);
                            } else {
                                // Still more to write, re-register for next write event
                                if let Err(e) = register_write_event(kq, fd) {
                                    error!("Failed to re-register write event for fd {fd}: {e}");
                                    write_buffers.remove(&fd); // Stop trying
                                }
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // This can happen if the buffer filled up again between the event
                            // and the write call. Re-register to try again later.
                            if let Err(e) = register_write_event(kq, fd) {
                                error!("Failed to re-register write event for fd {fd}: {e}");
                                write_buffers.remove(&fd); // Stop trying
                            }
                        }
                        Err(e) => {
                            error!("Error during buffered write on fd {fd}: {e}");
                            write_buffers.remove(&fd); // Stop trying
                        }
                    }
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
                            info!("Client fd {fd} closed connection");
                            client_server_configs.remove(&fd);
                            connection_buffers.remove(&fd);
                            client_addresses.remove(&fd);
                            server_addresses.remove(&fd);
                            write_buffers.remove(&fd);
                        }
                        Ok(n) => {
                            buffer.extend_from_slice(&chunk[..n]);

                            match crate::http::request::parse_request_from_buffer(buffer) {
                                Ok(Some((request, consumed))) => {
                                    let host = request
                                        .headers
                                        .get("host")
                                        .map(|s| s.as_str())
                                        .unwrap_or("-");
                                    if let (Some(remote_addr), Some(local_addr)) =
                                        (client_addresses.get(&fd), server_addresses.get(&fd))
                                    {
                                        info!(
                                            "Request from {} to {} (Host: {}): {} {}",
                                            remote_addr,
                                            local_addr,
                                            host,
                                            request.method,
                                            request.path
                                        );
                                    }

                                    connections_activity.insert(fd, Instant::now());
                                    let host_without_port = host.split(':').next().unwrap_or("");
                                    let server_config = server_configs
                                        .iter()
                                        .find(|c| c.server_name == host_without_port)
                                        .unwrap_or_else(|| &server_configs[0]);

                                    if request.body.len() > server_config.client_max_body_size {
                                        let payload_size = request.body.len();
                                        let max_size = server_config.client_max_body_size;
                                        if let Some(addr) = client_addresses.get(&fd) {
                                            error!("Client {} sent payload of {} bytes to {}, exceeding max body size of {} bytes",
                                            addr, payload_size, request.path, max_size);
                                        }
                                        let response = crate::http::response::Response::new(
                                            413,
                                            b"Payload Too Large".to_vec(),
                                            server_config,
                                            Some(&request),
                                        );
                                        if let Err(e) = schedule_response(
                                            fd,
                                            kq,
                                            &mut write_buffers,
                                            response.to_bytes(),
                                            &mut connections_activity,
                                        ) {
                                            error!("Failed to schedule 413 response for fd {fd}: {e}");
                                        }
                                    } else {
                                        let response = {
                                            let mut session_manager_lock =
                                                match session_manager.lock() {
                                                    Ok(guard) => guard,
                                                    Err(poisoned) => poisoned.into_inner(),
                                                };
                                            let mut session_id: Option<Uuid> = None;
                                            if let Some(cookie_header) =
                                                request.cookies.get("session_id")
                                            {
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
                                                debug!(
                                                    "No valid session found, creating a new one."
                                                );
                                                let new_session =
                                                    session_manager_lock.create_session();
                                                session_id = Some(new_session.id);
                                                current_session = session_manager_lock
                                                    .get_session(&new_session.id);
                                            }

                                            match crate::handler::find_route(
                                                &request,
                                                server_config,
                                            ) {
                                                Some(route) => {
                                                    let mut resp = crate::handler::handle_request(
                                                        &request,
                                                        route,
                                                        server_config,
                                                        &mut current_session,
                                                    );
                                                    if let Some(sid) = session_id {
                                                        if !request
                                                            .cookies
                                                            .contains_key("session_id")
                                                        {
                                                            resp.headers.insert(
                                                                "Set-Cookie".to_string(),
                                                                format!(
                                                                    "session_id={sid}; HttpOnly; Path=/"
                                                                ),
                                                            );
                                                        }
                                                    }
                                                    resp
                                                }
                                                None => crate::http::response::Response::new(
                                                    404,
                                                    b"Not Found".to_vec(),
                                                    server_config,
                                                    Some(&request),
                                                ),
                                            }
                                        };
                                        if let Err(e) = schedule_response(
                                            fd,
                                            kq,
                                            &mut write_buffers,
                                            response.to_bytes(),
                                            &mut connections_activity,
                                        ) {
                                            error!("Failed to schedule response for fd {fd}: {e}");
                                        }
                                    }

                                    buffer.drain(..consumed);
                                }
                                Ok(None) => {
                                    std::mem::forget(stream_owner);
                                }
                                Err(_) => {
                                    let response = crate::http::response::Response::new(
                                        400,
                                        b"Bad Request".to_vec(),
                                        &server_configs[0],
                                        None,
                                    );
                                    if let Err(e) = schedule_response(
                                        fd,
                                        kq,
                                        &mut write_buffers,
                                        response.to_bytes(),
                                        &mut connections_activity,
                                    ) {
                                        error!("Failed to schedule 400 response for fd {fd}: {e}");
                                    }
                                    client_server_configs.remove(&fd);
                                    connection_buffers.remove(&fd);
                                    client_addresses.remove(&fd);
                                    server_addresses.remove(&fd);
                                    write_buffers.remove(&fd);
                                }
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::mem::forget(stream_owner);
                        }
                        Err(e) => {
                            error!("Read error on fd {fd}: {e}");
                            client_server_configs.remove(&fd);
                            connection_buffers.remove(&fd);
                            client_addresses.remove(&fd);
                            server_addresses.remove(&fd);
                            write_buffers.remove(&fd);
                        }
                    }
                }
            } else if ev.flags & EV_EOF != 0 {
                info!("Client fd {} disconnected (EOF)", fd);
                unsafe {
                    libc::close(fd);
                    client_server_configs.remove(&fd);
                    connection_buffers.remove(&fd);
                    client_addresses.remove(&fd);
                    server_addresses.remove(&fd);
                    write_buffers.remove(&fd);
                }
            }
        }
    }
}

fn register_write_event(kq: i32, fd: i32) -> std::io::Result<()> {
    let change = libc::kevent {
        ident: fd as libc::uintptr_t,
        filter: libc::EVFILT_WRITE,
        flags: libc::EV_ADD | libc::EV_ONESHOT,
        fflags: 0,
        data: 0,
        udata: std::ptr::null_mut(),
    };
    kqueue::kevent(kq, std::slice::from_ref(&change), &mut [], None).map(|_| ())
}

fn schedule_response(
    fd: i32,
    kq: i32,
    write_buffers: &mut HashMap<i32, (usize, Vec<u8>)>, // (offset, data)
    response_bytes: Vec<u8>,
    connections_activity: &mut HashMap<i32, Instant>,
) -> std::io::Result<()> {
    if write_buffers.contains_key(&fd) {
        warn!("Dropping new response for fd {fd} as a write is already pending.");
        return Ok(());
    }

    let mut stream = unsafe { std::net::TcpStream::from_raw_fd(fd) };
    let result = match stream.write(&response_bytes) {
        Ok(n) => {
            connections_activity.insert(fd, Instant::now());
            if n < response_bytes.len() {
                write_buffers.insert(fd, (n, response_bytes));
                register_write_event(kq, fd)?;
            }
            Ok(())
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            connections_activity.insert(fd, Instant::now());
            write_buffers.insert(fd, (0, response_bytes));
            register_write_event(kq, fd)?;
            Ok(())
        }
        Err(e) => {
            error!("Initial write error on fd {fd}: {e}");
            Err(e)
        }
    };
    std::mem::forget(stream);
    result
}
