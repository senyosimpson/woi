use std::io;
use std::os::unix::prelude::AsRawFd;

use super::epoll::Interest;
use super::reactor::{Handle, IoSource};

// Async I/O adapter that bridges the event queue and I/O of interest
pub(crate) struct Pollable<T> {
    io: T,
    source: IoSource,
    handle: Handle,
}

impl<T: AsRawFd> Pollable<T> {
    pub fn new(io: T) -> io::Result<Self> {
        let interest = Interest::READABLE | Interest::WRITABLE;
        Self::new_with_interest(io, interest)
    }

    pub fn new_with_interest(io: T, interest: Interest) -> io::Result<Self> {
        let mut handle = Handle::current();
        let source = handle.register(io.as_raw_fd(), interest)?;
        Ok(Pollable { io, source, handle })
    }
}
