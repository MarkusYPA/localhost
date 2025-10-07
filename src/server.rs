use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::fd::{AsRawFd, FromRawFd};
use std::time::Duration;

use libc::{self, EV_ADD, EV_ENABLE, EV_EOF, EVFILT_READ};

use crate::config::ServerConfig;
use crate::io::kqueue; // your wrapper module

pub fn run(config: ServerConfig) -> std::io::Result<()> {
    // --- Setup listener ---
    let addr = format!("{}:{}", config.host, config.ports[0]);
    let listener = TcpListener::bind(&addr)?;
    listener.set_nonblocking(true)?;
    println!("Server listening on {}", addr);

    // --- Create kqueue ---
    let kq = kqueue::kqueue()?;
    println!("Server initialized, waiting for events...");

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
                            let req = String::from_utf8_lossy(&buf[..n]);
                            println!("Got request from fd {}:\n{}", fd, req);
                            let body = b"Hello, world!";
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                String::from_utf8_lossy(body)
                            );
                            let _ = stream.write_all(resp.as_bytes());
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
