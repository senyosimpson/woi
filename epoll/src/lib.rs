use libc;
use std::io;
use std::os::unix::io::RawFd;

/// Converts C error codes into a Rust Result type
fn cvt(result: i32) -> io::Result<i32> {
    if result < 0 {
        return Err(io::Error::last_os_error());
    } else {
        return Ok(result);
    }
}

/// Safe wrapper around `libc::epoll_create`
#[cfg(target_os = "linux")]
fn create(size: i32) -> io::Result<RawFd> {
    cvt(unsafe { libc::epoll_create(size) })
}

/// Safe wrapper around `libc::epoll_create1`
#[cfg(target_os = "linux")]
fn create1(flags: i32) -> io::Result<RawFd> {
    cvt(unsafe { libc::epoll_create1(flags) })
}

/// Safe wrapper around `libc::epoll_ctl`
#[cfg(target_os = "linux")]
fn ctl(epfd: RawFd, op: i32, fd: RawFd, event: *mut libc::epoll_event) -> io::Result<()> {
    cvt(unsafe { libc::epoll_ctl(epfd, op, fd, event) })?;
    Ok(())
}

/// Safe wrapper around `libc::epoll_wait`
/// https://man7.org/linux/man-pages/man2/epoll_wait.2.html
#[cfg(target_os = "linux")]
fn wait(epfd: RawFd, events: &mut [libc::epoll_event], timeout: i32) -> io::Result<i32> {
    cvt(unsafe { libc::epoll_wait(epfd, events.as_mut_ptr(), events.len() as i32, timeout) })
}

/// Safe wrapper around `libc::close`
/// https://man7.org/linux/man-pages/man2/close.2.html
#[cfg(target_os = "linux")]
fn close(fd: RawFd) -> io::Result<()> {
    cvt(unsafe { libc::close(fd) })?;
    Ok(())
}

#[cfg(test)]
mod tests {
}
