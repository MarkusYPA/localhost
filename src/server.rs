use std::collections::HashMap;
use std::net::TcpListener;
use std::os::unix::io::{AsRawFd, RawFd};
use std::time::Duration;

use crate::config::ServerConfig;
use crate::io::kqueue;

pub enum ConnectionState {
    ReadingRequestLine,
    ReadingHeaders,
    ReadingBody { expected: usize, received: usize },
    Processing,
    WritingResponse { sent: usize },
    KeepAlive,
}

pub struct Connection {
    pub fd: RawFd,
    pub state: ConnectionState,
    pub buffer: Vec<u8>,
    pub response: Vec<u8>,
}

pub fn run(config: ServerConfig) -> Result<(), std::io::Error> {
    let listener = TcpListener::bind(format!("{}:{}", config.host, config.ports[0]))?;
    listener.set_nonblocking(true)?;
    let listener_fd = listener.as_raw_fd();

    let kq = kqueue::kqueue()?;

    let changelist = [libc::kevent {
        ident: listener_fd as usize,
        filter: libc::EVFILT_READ,
        flags: libc::EV_ADD | libc::EV_ENABLE,
        fflags: 0,
        data: 0,
        udata: 0 as *mut libc::c_void,
    }];

    kqueue::kevent(kq, &changelist, &mut [], Some(Duration::from_secs(0)))?;

    let mut connections = HashMap::<RawFd, Connection>::new();
    let mut events = Vec::with_capacity(1024);
    /* let mut events = vec![
        libc::kevent {
            ident: 0,
            filter: 0,
            flags: 0,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        1024
    ]; */

    loop {
        events.clear();
        let num_events = kqueue::kevent(kq, &[], &mut events, None)?;

        for i in 0..num_events as usize {
            let event = unsafe { events.get_unchecked(i) };
            let event_fd = event.ident as RawFd;

            println!("Listening on {}:{}", config.host, config.ports[0]);
            println!("kqueue fd: {}", kq);
            println!("listener fd: {}", listener_fd);

            if event_fd == listener_fd {
                println!("accepting connection");

                // Accept new connections
                match listener.accept() {
                    Ok((stream, _)) => {
                        let client_fd = stream.as_raw_fd();
                        stream.set_nonblocking(true)?;

                        let changelist = [libc::kevent {
                            ident: client_fd as usize,
                            filter: libc::EVFILT_READ,
                            flags: libc::EV_ADD | libc::EV_ENABLE,
                            fflags: 0,
                            data: 0,
                            udata: 0 as *mut libc::c_void,
                        }];

                        kqueue::kevent(kq, &changelist, &mut [], Some(Duration::from_secs(0)))?;

                        connections.insert(
                            client_fd,
                            Connection {
                                fd: client_fd,
                                state: ConnectionState::ReadingRequestLine,
                                buffer: Vec::new(),
                                response: Vec::new(),
                            },
                        );
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // Spurious wakeup, ignore
                    }
                    Err(e) => return Err(e),
                }
            } else {
                println!("reading or writing");

                if event.filter == libc::EVFILT_READ {
                    println!("Read event");
                    let conn = connections.get_mut(&(event_fd as i32)).unwrap();
                    let mut buffer = [0u8; 1024];
                    match unsafe {
                        libc::read(conn.fd, buffer.as_mut_ptr() as *mut libc::c_void, 1024)
                    } {
                        -1 => {
                            // Error
                            connections.remove(&(event_fd as i32));
                        }
                        0 => {
                            // Connection closed
                            connections.remove(&(event_fd as i32));
                        }
                        n => {
                            // Data received
                            println!("{}", String::from_utf8_lossy(&buffer[..n as usize]));
                            //conn.response = b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, world!".to_vec();
                            conn.response = b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\nConnection: close\r\n\r\nHello, world!".to_vec();

                            let changelist = [libc::kevent {
                                ident: conn.fd as usize,
                                filter: libc::EVFILT_WRITE,
                                flags: libc::EV_ADD | libc::EV_ENABLE,
                                fflags: 0,
                                data: 0,
                                udata: 0 as *mut libc::c_void,
                            }];
                            kqueue::kevent(kq, &changelist, &mut [], Some(Duration::from_secs(0)))
                                .unwrap();
                            println!("Write event registered");
                        }
                    }
                } else if event.filter == libc::EVFILT_WRITE {
                    println!("Write event");
                    let conn = connections.get_mut(&(event_fd as i32)).unwrap();

                    match unsafe {
                        libc::write(
                            conn.fd,
                            conn.response.as_ptr() as *const libc::c_void,
                            conn.response.len(),
                        )
                    } {
                        -1 => {
                            // Error
                            connections.remove(&(event_fd as i32));
                        }
                        n => {
                            // Data sent
                            if n as usize == conn.response.len() {
                                // All data sent, close connection
                                std::thread::sleep(std::time::Duration::from_millis(10)); // small delay after the write (for testing)
                                unsafe { libc::close(conn.fd) };
                                connections.remove(&(event_fd as i32));
                            }
                        }
                    }
                }
            }
        }
    }
}
