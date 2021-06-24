use std::{
    io,
    os::unix::io::{AsRawFd, RawFd},
    time::Duration,
};

use super::epoll::{self, CtlOp, Event, Events, Interest, Token};

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
        let fd = epoll::create()?;

        let poll = Poll { fd };
        Ok(poll)
    }

    pub fn add(&self, source: impl Source, interest: Interest, token: Token) -> io::Result<()> {
        let event = Event::new(interest, token);
        epoll::ctl(self.fd, CtlOp::ADD, source.raw_fd(), Some(event))?;
        Ok(())
    }

    pub fn delete(&self, source: impl Source) -> io::Result<()> {
        epoll::ctl(self.fd, CtlOp::DEL, source.raw_fd(), None)?;
        Ok(())
    }
    pub fn modify(&self, source: impl Source, interest: Interest, token: Token) -> io::Result<()> {
        let event = Event::new(interest, token);
        epoll::ctl(self.fd, CtlOp::MOD, source.raw_fd(), Some(event))?;
        Ok(())
    }

    pub fn poll(&self, events: &mut Events, timeout: Option<Duration>) -> io::Result<()> {
        events.clear();
        let timeout = match timeout {
            Some(duration) => duration.as_millis() as i32,
            None => -1,
        };
        let n_events = epoll::wait(self.fd, events, timeout)?;

        // This is actually safe to call because `epoll::wait` returns the
        // number of events that were returned. Got this from Mio:
        // https://github.com/tokio-rs/mio/blob/22e885859bb481ae4c2827ab48552c3159fcc7f8/src/sys/unix/selector/epoll.rs#L77
        unsafe { events.set_len(n_events as usize) };
        Ok(())
    }
}