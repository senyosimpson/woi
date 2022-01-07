// A safe library for interacting with epoll

use std::{
    io,
    os::unix::io::{AsRawFd, RawFd},
    ptr,
    time::Duration,
};

use bitflags::bitflags;
use libc;

// The enum representation is changed to libc::c_int so that it works with
// libc epoll bindings.
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
pub struct Token(pub usize);

pub type Events = Vec<Event>;

// Uses #[repr(C)] to be interoperable with C. Learn more here:
// https://doc.rust-lang.org/reference/type-layout.html#the-c-representation
//
// I have not spent enough time learning about packed data structures. From
// what I've gathered, it's a way to efficiently use memory by removing
// padding. Learn more here:
//  - https://doc.rust-lang.org/reference/type-layout.html#the-alignment-modifiers
//  - https://www.mikroe.com/blog/packed-structures-make-memory-feel-safe
//
/// An equivalent of `libc::epoll_data`
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Event {
    interest: u32,
    data: u64,
}

impl Event {
    pub fn new(interest: Interest, token: Token) -> Event {
        Event {
            interest: interest.bits(),
            data: token.0 as u64,
        }
    }

    pub fn token(&self) -> Token {
        Token(self.data as usize)
    }
}

// TODO: Write documentation
bitflags! {
    pub struct Interest: u32 {
        const READABLE       = (libc::EPOLLET  | libc::EPOLLIN | libc::EPOLLRDHUP) as u32;
        const WRITABLE       = (libc::EPOLLET  | libc::EPOLLOUT) as u32;
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
    let event = match &mut event {
        Some(event) => event as *mut Event as *mut libc::epoll_event,
        None => ptr::null_mut(),
    };
    cvt(unsafe { libc::epoll_ctl(epfd, op as i32, fd, event) })?;
    Ok(())
}

/// Safe wrapper around `libc::epoll_wait`
/// Official documentation: https://man7.org/linux/man-pages/man2/epoll_wait.2.html
#[cfg(target_os = "linux")]
pub(crate) fn wait(epfd: RawFd, events: &mut Events, timeout: i32) -> io::Result<i32> {
    let capacity = events.capacity() as i32;
    let events = events.as_mut_ptr() as *mut libc::epoll_event;
    cvt(unsafe { libc::epoll_wait(epfd, events, capacity, timeout) })
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

impl<T: AsRawFd> Source for &T {
    fn raw_fd(&self) -> RawFd {
        self.as_raw_fd()
    }
}

pub struct Epoll {
    fd: RawFd,
}

impl Epoll {
    pub fn new() -> io::Result<Epoll> {
        let fd = create()?;
        let poll = Epoll { fd };
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
            None => -1, // TThis blocks indefinitely
        };
        let n_events = wait(self.fd, events, timeout)?;

        // This is actually safe to call because `epoll::wait` returns the
        // number of events that were returned. Got this from Mio:
        // https://github.com/tokio-rs/mio/blob/22e885859bb481ae4c2827ab48552c3159fcc7f8/src/sys/unix/selector/epoll.rs#L77
        unsafe { events.set_len(n_events as usize) };
        Ok(())
    }

    pub fn close(&self) -> io::Result<()> {
        close(self.fd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_epoll_instance() {
        // Test it works by creating an instance of epoll and then closing it
        // If this function does not work, it will panic
        let epoll = Epoll::new().unwrap();
        // Drop the error so we have a gaurantee that if the test fails, it is from
        // creating the epoll instance. This is arguably shady
        let _ = epoll.close();
    }

    #[test]
    fn add_event() {
        use std::net::TcpListener;
        use std::os::unix::io::AsRawFd;

        let epoll = Epoll::new().unwrap();
        let interest = Interest::READABLE | Interest::WRITABLE;
        let listener = TcpListener::bind("localhost:3000").unwrap();

        epoll.add(listener.as_raw_fd(), interest, Token(1)).unwrap();
        let _ = epoll.close();
    }

    #[test]
    fn poll_event() {
        use std::io::Write;
        use std::net::{TcpListener, TcpStream};
        use std::os::unix::io::AsRawFd;

        let epoll = Epoll::new().unwrap();
        let interest = Interest::READABLE;

        let listener = TcpListener::bind("localhost:3000").unwrap();
        epoll.add(listener.as_raw_fd(), interest, Token(1)).unwrap();

        let mut socket = TcpStream::connect("localhost:3000").unwrap();
        let request = "Hello world!";
        socket.write_all(request.as_bytes()).unwrap();

        let maxevents = 10;
        let mut events = Events::with_capacity(maxevents);
        epoll.poll(&mut events, None).unwrap();
        epoll.close().unwrap();

        assert_eq!(events.len(), 1);
    }
}
