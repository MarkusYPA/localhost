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
        tv_sec: d.as_secs() as libc::time_t,
        tv_nsec: d.subsec_nanos() as libc::c_long,
    });

    let changep = if changelist.is_empty() {
        std::ptr::null()
    } else {
        changelist.as_ptr()
    };

    let eventp = if eventlist.is_empty() {
        std::ptr::null_mut()
    } else {
        eventlist.as_mut_ptr()
    };

    let ret = unsafe {
        libc::kevent(
            kq,
            changep,
            changelist.len() as i32,
            eventp,
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

pub fn wait_for_pid(pid: i32, timeout: Duration) -> Result<bool, std::io::Error> {
    let kq = kqueue()?;
    let event = libc::kevent {
        ident: pid as usize,
        filter: libc::EVFILT_PROC,
        flags: libc::EV_ADD | libc::EV_ONESHOT,
        fflags: libc::NOTE_EXIT,
        data: 0,
        udata: std::ptr::null_mut(),
    };

    let mut events = [libc::kevent { ..event }];
    let result = kevent(kq, &[event], &mut events, Some(timeout))?;

    unsafe { libc::close(kq) };

    Ok(result > 0)
}
