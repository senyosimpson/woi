use epoll;
use std::{io, os::unix::io::RawFd};

#[derive(Debug, PartialEq, Eq)]
pub struct Token(u64);

pub struct Registry;

impl Registry {
    pub fn new() -> Registry {}
}

pub struct Poll {
    fd: RawFd,
    registry: Registry
}

impl Poll {
    pub fn new() {}
    pub fn register() {}
    pub fn deregister() {}
    pub fn reregister() {}
    pub fn poll() {}
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
