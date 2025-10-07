use std::collections::HashMap;
use std::net::TcpListener;
use std::os::unix::io::{AsRawFd, RawFd};
use std::time::Duration;

use crate::config::ServerConfig;
use crate::io::kqueue;

pub enum ConnectionState {
    Reading,
    Writing,
}

pub struct Connection {
    pub fd: RawFd,
    pub state: ConnectionState,
    pub response: Vec<u8>,
}

pub fn run(config: ServerConfig) -> Result<(), std::io::Error> {
    let addr = format!("{}:{}", config.host, config.ports[0]);
    let listener = TcpListener::bind(&addr)?;
    listener.set_nonblocking(true)?;
    let listener_fd = listener.as_raw_fd();

    println!("Server listening on {}", addr);

    // Create the kqueue
    let kq = kqueue::kqueue()?;

    // Register the listener fd for read events
    let changelist = [libc::kevent {
        ident: listener_fd as usize,
        filter: libc::EVFILT_READ,
        flags: libc::EV_ADD | libc::EV_ENABLE,
        fflags: 0,
        data: 0,
        udata: std::ptr::null_mut(),
    }];

    // Submit changelist to kernel (no output expected)
    kqueue::kevent(kq, &changelist, &mut [], Some(Duration::from_secs(0)))?;

    // Event buffer and connection map
    let mut events = vec![
        libc::kevent {
            ident: 0,
            filter: 0,
            flags: 0,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        1024
    ];
    let mut connections: HashMap<RawFd, Connection> = HashMap::new();

    println!("Server initialized, waiting for events...");

    loop {
        // Wait for events (blocking until something happens)
        let num_events = kqueue::kevent(kq, &[], &mut events, None)?;

        for i in 0..num_events {
            let ev = events[i as usize];
            let event_fd = ev.ident as RawFd;

            // New incoming connection?
            if event_fd == listener_fd {
                match listener.accept() {
                    Ok((stream, addr)) => {
                        stream.set_nonblocking(true)?;
                        let fd = stream.as_raw_fd();
                        println!("Accepted connection from {}", addr);

                        let changelist = [libc::kevent {
                            ident: fd as usize,
                            filter: libc::EVFILT_READ,
                            flags: libc::EV_ADD | libc::EV_ENABLE,
                            fflags: 0,
                            data: 0,
                            udata: std::ptr::null_mut(),
                        }];
                        kqueue::kevent(kq, &changelist, &mut [], None)?;
                        println!("Registered client fd {} for read events", fd);

                        connections.insert(
                            fd,
                            Connection {
                                fd,
                                state: ConnectionState::Reading,
                                response: Vec::new(),
                            },
                        );
                    }
                    Err(e) => eprintln!("Accept error: {}", e),
                }
            } else if ev.filter == libc::EVFILT_READ {
                println!("Read event on fd {}", event_fd);

                let mut buffer = [0u8; 1024];
                let n = unsafe {
                    libc::read(
                        event_fd,
                        buffer.as_mut_ptr() as *mut libc::c_void,
                        buffer.len(),
                    )
                };
                if n <= 0 {
                    println!("Client closed fd {}", event_fd);
                    unsafe { libc::close(event_fd) };
                    connections.remove(&event_fd);
                    continue;
                }

                let req = String::from_utf8_lossy(&buffer[..n as usize]);
                println!("Received request:\n{}", req);

                let conn = connections.get_mut(&event_fd).unwrap();
                conn.response = b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\nConnection: close\r\n\r\nHello, world!"
                    .to_vec();

                // Switch to write events
                let changelist = [libc::kevent {
                    ident: event_fd as usize,
                    filter: libc::EVFILT_WRITE,
                    flags: libc::EV_ADD | libc::EV_ENABLE,
                    fflags: 0,
                    data: 0,
                    udata: std::ptr::null_mut(),
                }];
                kqueue::kevent(kq, &changelist, &mut [], Some(Duration::from_secs(0)))?;
            } else if ev.filter == libc::EVFILT_WRITE {
                println!("Write event on fd {}", event_fd);

                if let Some(conn) = connections.remove(&event_fd) {
                    let _ = unsafe {
                        libc::write(
                            conn.fd,
                            conn.response.as_ptr() as *const libc::c_void,
                            conn.response.len(),
                        )
                    };
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    unsafe { libc::close(conn.fd) };
                    println!("Closed fd {}", event_fd);
                }
            }
        }
    }
}
