use libc;
use std::io;
use std::os::unix::io::RawFd;

/// Control options for `epoll_ctl`
#[repr(i32)]
enum EpollCtlOp {
    ADD = libc::EPOLL_CTL_ADD,
    MOD = libc::EPOLL_CTL_MOD,
    DEL = libc::EPOLL_CTL_DEL,
}

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
/// https://man7.org/linux/man-pages/man2/epoll_ctl.2.html
#[cfg(target_os = "linux")]
fn ctl(epfd: RawFd, op: EpollCtlOp, fd: RawFd, event: *mut libc::epoll_event) -> io::Result<()> {
    cvt(unsafe { libc::epoll_ctl(epfd, op as i32, fd, event) })?;
    Ok(())
}

/// Safe wrapper around `libc::epoll_wait`
/// https://man7.org/linux/man-pages/man2/epoll_wait.2.html
#[cfg(target_os = "linux")]
fn wait(epfd: RawFd, events: &mut [libc::epoll_event], maxevents: i32, timeout: i32) -> io::Result<i32> {
    cvt(unsafe { libc::epoll_wait(epfd, events.as_mut_ptr(), maxevents, timeout) })
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
    use super::*;
    #[test]
    fn create_epoll_queue() {
        // Test it works by creating an instance of epoll and then closing it
        // If this function does not work, it will panic
        let queue = create(1).unwrap();
        close(queue).unwrap();
    }

    #[test]
    fn add_event() {
        use std::net::TcpStream;
        use std::os::unix::io::AsRawFd;

        let queue = create(1).unwrap();
        let events = libc::EPOLLIN | libc::EPOLLONESHOT;
        let mut event = libc::epoll_event {
            events: events as u32,
            u64: 1,
        };

        let socket = TcpStream::connect("localhost:3000").unwrap();
        ctl(queue, EpollCtlOp::ADD, socket.as_raw_fd(), &mut event).unwrap();
        close(queue).unwrap();
    }

    #[test]
    fn wait_for_event() {
        use std::io::Write;
        use std::net::TcpStream;
        use std::os::unix::io::AsRawFd;

        let queue = create(1).unwrap();
        let events = libc::EPOLLIN | libc::EPOLLONESHOT;
        let mut event = libc::epoll_event {
            events: events as u32,
            u64: 1,
        };

        // I need an actual way of testing this without spinning up an entire server. I can potentially
        // query an actual website.
        let mut socket = TcpStream::connect("localhost:3000").unwrap();
        let request = "GET /delay HTTP/1.1\r\nHost: localhost:3000\r\nConnection: close\r\n\r\n";
        socket.write_all(request.as_bytes()).unwrap();

        ctl(queue, EpollCtlOp::ADD, socket.as_raw_fd(), &mut event).unwrap();

        let maxevents = 10;
        let mut events: Vec<libc::epoll_event> = Vec::with_capacity(maxevents);
        loop {
            let num_events = wait(queue, &mut events, maxevents as i32, -1).unwrap();
            println!("Received {} number of events!", num_events);
            close(queue).unwrap();

            // This is very ugly. Think of a cleaner way of handling this.
            assert_eq!(num_events, 1);
            break;
        }
    }
}
