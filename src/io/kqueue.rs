use libc;
use std::os::unix::io::RawFd;
use std::time::Duration;

pub fn kqueue() -> Result<RawFd, std::io::Error> {
    let fd = unsafe { libc::kqueue() };
    if fd < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(fd)
    }
}

pub fn kevent(
    kq: RawFd,
    changelist: &[libc::kevent],
    eventlist: &mut [libc::kevent],
    timeout: Option<Duration>,
) -> Result<i32, std::io::Error> {
    let timeout = timeout.map(|d| libc::timespec {
        tv_sec: d.as_secs() as i64,
        tv_nsec: d.subsec_nanos() as i64,
    });

    let ret = unsafe {
        libc::kevent(
            kq,
            changelist.as_ptr(),
            changelist.len() as i32,
            eventlist.as_mut_ptr(),
            eventlist.len() as i32,
            timeout.as_ref().map_or(std::ptr::null(), |t| t as *const _),
        )
    };

    if ret < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(ret)
    }
}