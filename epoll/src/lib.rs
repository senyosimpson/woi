use std::io;
use std::os::unix::io::RawFd;

use libc;
use bitflags::bitflags;

/// Control options for `epoll_ctl`
#[repr(i32)]
pub enum CtlOp {
    /// Adds an entry to the interest list
    ADD = libc::EPOLL_CTL_ADD,
    /// Change the settings of an associated entry in the interest list
    MOD = libc::EPOLL_CTL_MOD,
    /// Removes an entry from the interest list
    DEL = libc::EPOLL_CTL_DEL,
}

/// An equivalent of `libc::epoll_create`
#[repr(C)]
#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct Event {
    events: u32,
    data: u64,
}

impl Event {
    pub fn new(events: Events, data: u64) -> Event {
        Event {
            events: events.bits(),
            data: data
        }
    }
}

// TODO: Write documentation
bitflags! {
    pub struct Events: u32 {
        const EPOLLIN        = libc::EPOLLIN as u32;
        const EPOLLOUT       = libc::EPOLLOUT as u32;
        const EPOLLRDHUP     = libc::EPOLLRDHUP as u32;
        const EPOLLPRI       = libc::EPOLLPRI as u32;
        const EPOLLERR       = libc::EPOLLERR as u32;
        const EPOLLHUP       = libc::EPOLLHUP as u32;
        const EPOLLET        = libc::EPOLLET as u32;
        const EPOLLONESHOT   = libc::EPOLLONESHOT as u32;
        const EPOLLWAKEUP    = libc::EPOLLWAKEUP as u32;
        const EPOLLEXCLUSIVE = libc::EPOLLEXCLUSIVE as u32;
    }
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
/// Manpages: https://man7.org/linux/man-pages/man2/epoll_create.2.html
#[cfg(target_os = "linux")]
pub fn create(size: i32) -> io::Result<RawFd> {
    cvt(unsafe { libc::epoll_create(size) })
}

/// Safe wrapper around `libc::epoll_create1`
/// Manpages: https://man7.org/linux/man-pages/man2/epoll_create1.2.html
#[cfg(target_os = "linux")]
pub fn create1(flags: i32) -> io::Result<RawFd> {
    cvt(unsafe { libc::epoll_create1(flags) })
}

/// Safe wrapper around `libc::epoll_ctl`
/// Manpages: https://man7.org/linux/man-pages/man2/epoll_ctl.2.html
#[cfg(target_os = "linux")]
pub fn ctl(epfd: RawFd, op: CtlOp, fd: RawFd, event: &mut Event) -> io::Result<()> {
    let event_ptr: *mut Event = event;
    cvt(unsafe { libc::epoll_ctl(epfd, op as i32, fd, event_ptr as *mut libc::epoll_event) })?;
    Ok(())
}

/// Safe wrapper around `libc::epoll_wait`
/// Manpages: https://man7.org/linux/man-pages/man2/epoll_wait.2.html
#[cfg(target_os = "linux")]
pub fn wait(epfd: RawFd, events: &mut [Event], maxevents: i32, timeout: i32) -> io::Result<i32> {
    let events_ptr = events.as_mut_ptr() as *mut libc::epoll_event;
    cvt(unsafe { libc::epoll_wait(epfd, events_ptr, maxevents, timeout) })
}

/// Safe wrapper around `libc::close`
/// Manpages: https://man7.org/linux/man-pages/man2/close.2.html
#[cfg(target_os = "linux")]
pub fn close(fd: RawFd) -> io::Result<()> {
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
        let events = Events::EPOLLIN | Events::EPOLLONESHOT;
        let mut event = Event::new(events, 1);

        let socket = TcpStream::connect("localhost:3000").unwrap();
        ctl(queue, CtlOp::ADD, socket.as_raw_fd(), &mut event).unwrap();
        close(queue).unwrap();
    }

    #[test]
    fn wait_for_event() {
        use std::io::Write;
        use std::net::TcpStream;
        use std::os::unix::io::AsRawFd;

        let queue = create(1).unwrap();
        let events = Events::EPOLLIN | Events::EPOLLONESHOT;
        let mut event = Event::new(events, 1);


        // I need an actual way of testing this without spinning up an entire server. I can potentially
        // query an actual website.
        let mut socket = TcpStream::connect("localhost:3000").unwrap();
        let request = "GET /delay HTTP/1.1\r\nHost: localhost:3000\r\nConnection: close\r\n\r\n";
        socket.write_all(request.as_bytes()).unwrap();

        ctl(queue, CtlOp::ADD, socket.as_raw_fd(), &mut event).unwrap();

        let maxevents = 10;
        let mut events: Vec<Event> = Vec::with_capacity(maxevents);
        let num_events = wait(queue, &mut events, maxevents as i32, -1).unwrap();
        println!("Received {} number of events!", num_events);
        close(queue).unwrap();

        assert_eq!(num_events, 1);
    }
}
