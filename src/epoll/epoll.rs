use std::{
    io,
    os::unix::io::{AsRawFd, RawFd},
    ptr,
    time::Duration,
};

use bitflags::bitflags;
use libc;

// The enum representation is changed to i32 so that it works with
// libc epoll bindings. An alternative way to write this is as
// #[repr(libc::c_int)] since libc::c_int = i32
//
// Learn more about type layout representations:
// https://doc.rust-lang.org/reference/type-layout.html#representations

/// Control options for `epoll_ctl`
#[repr(i32)]
pub(crate) enum CtlOp {
    /// Adds an entry to the interest list
    ADD = libc::EPOLL_CTL_ADD,
    /// Change the settings of an associated entry in the interest list
    MOD = libc::EPOLL_CTL_MOD,
    /// Removes an entry from the interest list
    DEL = libc::EPOLL_CTL_DEL,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Token(pub u64);

// Uses #[repr(C)] to be interoperable with C. Learn more here:
// https://doc.rust-lang.org/reference/type-layout.html#the-c-representation
//
// I have not spent enough time learning about packed data structures. From
// what I've gathered, it's a way to efficiently use memory by removing
// padding. Learn more here:
// https://doc.rust-lang.org/reference/type-layout.html#the-alignment-modifiers
// https://www.mikroe.com/blog/packed-structures-make-memory-feel-safe
/// An equivalent of `libc::epoll_data`
#[repr(C)]
#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct Event {
    interest: u32,
    data: u64,
}

impl Event {
    pub fn new(interest: Interest, token: Token) -> Event {
        Event {
            interest: interest.bits(),
            data: token.0,
        }
    }

    pub fn empty() -> Event {
        Event {
            interest: 0,
            data: 0,
        }
    }
    pub fn token(&self) -> Token {
        Token(self.data)
    }
}

pub type Events = Vec<Event>;

// TODO: Write documentation
bitflags! {
    pub struct Interest: u32 {
        const READABLE       = (libc::EPOLLIN  | libc::EPOLLONESHOT) as u32;
        const WRITABLE       = (libc::EPOLLOUT | libc::EPOLLONESHOT) as u32;
        // const EPOLLRDHUP     = libc::EPOLLRDHUP as u32;
        // const EPOLLPRI       = libc::EPOLLPRI as u32;
        // const EPOLLERR       = libc::EPOLLERR as u32;
        // const EPOLLHUP       = libc::EPOLLHUP as u32;
        // const EPOLLET        = libc::EPOLLET as u32;
        // const EPOLLWAKEUP    = libc::EPOLLWAKEUP as u32;
        // const EPOLLEXCLUSIVE = libc::EPOLLEXCLUSIVE as u32;
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

/// Safe wrapper around `libc::epoll_create1`
/// Uses `epoll_create1` by default with the close-on-exec flag set
/// Official documentation: https://man7.org/linux/man-pages/man2/epoll_create.2.html
#[cfg(target_os = "linux")]
pub(crate) fn create() -> io::Result<RawFd> {
    cvt(unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) })
}

/// Safe wrapper around `libc::epoll_ctl`
/// Official documentation: https://man7.org/linux/man-pages/man2/epoll_ctl.2.html
#[cfg(target_os = "linux")]
pub(crate) fn ctl(epfd: RawFd, op: CtlOp, fd: RawFd, mut event: Option<Event>) -> io::Result<()> {
    // This uses a neat trick to work with epoll. We get a mutable pointer
    // to the event struct. Since it is always valid to cast a pointer to
    // any other type, we can convert it into a `libc::epoll_event` pointer.
    // However, it is not safe to dereference. Imagine the structure event and
    // libc::epoll_event were different - the data returned would be corrupted.
    // Therefore, it is up to us to ensure that this is actually valid. We also
    // have to wrap it in an unsafe block.
    let event = if let Some(ev) = &mut event {
        let ev_ptr: *mut Event = ev;
        let ev_ptr = ev_ptr as *mut libc::epoll_event;
        ev_ptr
    } else {
        ptr::null_mut()
    };

    cvt(unsafe { libc::epoll_ctl(epfd, op as i32, fd, event) })?;
    Ok(())
}

/// Safe wrapper around `libc::epoll_wait`
/// Official documentation: https://man7.org/linux/man-pages/man2/epoll_wait.2.html
#[cfg(target_os = "linux")]
pub(crate) fn wait(epfd: RawFd, events: &mut Events, timeout: i32) -> io::Result<i32> {
    let events_ptr = events.as_mut_ptr() as *mut libc::epoll_event;
    cvt(unsafe { libc::epoll_wait(epfd, events_ptr, events.capacity() as i32, timeout) })
}

/// Safe wrapper around `libc::close`
/// Official documentation: https://man7.org/linux/man-pages/man2/close.2.html
#[cfg(target_os = "linux")]
pub(crate) fn close(fd: RawFd) -> io::Result<()> {
    cvt(unsafe { libc::close(fd) })?;
    Ok(())
}

pub trait Source {
    fn raw_fd(&self) -> RawFd;
}

impl Source for RawFd {
    fn raw_fd(&self) -> RawFd {
        *self
    }
}

// This is essentially a way to implement a trait on another
// trait through using trait bounds. We read the below as:
// implement trait Source for all types T that implement AsRawFd.
impl<T: AsRawFd> Source for &T {
    fn raw_fd(&self) -> RawFd {
        self.as_raw_fd()
    }
}

pub struct Poll {
    fd: RawFd,
}

impl Poll {
    pub fn new() -> io::Result<Poll> {
        let fd = create()?;

        let poll = Poll { fd };
        Ok(poll)
    }

    pub fn add(&self, source: impl Source, interest: Interest, token: Token) -> io::Result<()> {
        let event = Event::new(interest, token);
        ctl(self.fd, CtlOp::ADD, source.raw_fd(), Some(event))?;
        Ok(())
    }

    pub fn delete(&self, source: impl Source) -> io::Result<()> {
        ctl(self.fd, CtlOp::DEL, source.raw_fd(), None)?;
        Ok(())
    }
    pub fn modify(&self, source: impl Source, interest: Interest, token: Token) -> io::Result<()> {
        let event = Event::new(interest, token);
        ctl(self.fd, CtlOp::MOD, source.raw_fd(), Some(event))?;
        Ok(())
    }

    pub fn poll(&self, events: &mut Events, timeout: Option<Duration>) -> io::Result<()> {
        events.clear();
        let timeout = match timeout {
            Some(duration) => duration.as_millis() as i32,
            None => -1,
        };
        let n_events = wait(self.fd, events, timeout)?;

        // This is actually safe to call because `epoll::wait` returns the
        // number of events that were returned. Got this from Mio:
        // https://github.com/tokio-rs/mio/blob/22e885859bb481ae4c2827ab48552c3159fcc7f8/src/sys/unix/selector/epoll.rs#L77
        unsafe { events.set_len(n_events as usize) };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn create_epoll_queue() {
        // Test it works by creating an instance of epoll and then closing it
        // If this function does not work, it will panic
        let queue = create().unwrap();
        close(queue).unwrap();
    }

    #[test]
    fn add_event() {
        use std::net::TcpStream;
        use std::os::unix::io::AsRawFd;

        let queue = create().unwrap();
        let interest = Interest::READABLE | Interest::WRITABLE;
        let event = Event::new(interest, Token(1));

        let socket = TcpStream::connect("localhost:3000").unwrap();
        ctl(queue, CtlOp::ADD, socket.as_raw_fd(), Some(event)).unwrap();
        close(queue).unwrap();
    }

    #[test]
    fn wait_for_event() {
        use std::io::Write;
        use std::net::TcpStream;
        use std::os::unix::io::AsRawFd;

        let queue = create().unwrap();
        let interest = Interest::READABLE;
        let event = Event::new(interest, Token(1));

        // I need an actual way of testing this without spinning up an entire server.
        // I can potentially query an actual website.
        let mut socket = TcpStream::connect("localhost:3000").unwrap();
        let request = "GET /delay HTTP/1.1\r\nHost: localhost:3000\r\nConnection: close\r\n\r\n";
        socket.write_all(request.as_bytes()).unwrap();

        ctl(queue, CtlOp::ADD, socket.as_raw_fd(), Some(event)).unwrap();

        let maxevents = 10;
        let mut events = Events::with_capacity(maxevents);
        let num_events = wait(queue, &mut events, -1).unwrap();
        println!("Received {} number of events!", num_events);
        close(queue).unwrap();

        assert_eq!(num_events, 1);
    }
}
