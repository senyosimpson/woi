use std::{
    io,
    os::unix::io::{AsRawFd, RawFd},
    time::Duration,
};

mod epoll;
use epoll::CtlOp;
pub use epoll::{Event, Events, Interest, Token};

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
        let mut event = Event::new(interest, token);
        epoll::ctl(self.fd, CtlOp::ADD, source.raw_fd(), &mut event)?;
        Ok(())
    }

    pub fn delete(&self, source: impl Source) -> io::Result<()> {
        // Create an event to pass in but it is ignored
        // NOTE: Because it is ignored, we can pass in a token that's
        // already in use
        let mut event = Event::empty();
        epoll::ctl(self.fd, CtlOp::DEL, source.raw_fd(), &mut event)?;
        Ok(())
    }
    pub fn modify(&self, source: impl Source, interest: Interest, token: Token) -> io::Result<()> {
        let mut event = Event::new(interest, token);
        epoll::ctl(self.fd, CtlOp::MOD, source.raw_fd(), &mut event)?;
        Ok(())
    }

    pub fn poll(&self, events: &mut Events, timeout: Option<Duration>) -> io::Result<()> {
        events.clear();
        let timeout = match timeout {
            Some(duration) => duration.as_millis() as i32,
            None => -1,
        };
        let n_events = epoll::wait(self.fd, events, timeout)?;
        // This is actually safe to call because epoll::wait returns the
        // number of events that were returned
        // Got this from Mio: https://github.com/tokio-rs/mio/blob/22e885859bb481ae4c2827ab48552c3159fcc7f8/src/sys/unix/selector/epoll.rs#L77
        unsafe { events.set_len(n_events as usize) };
        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn add_event() {}
// }
