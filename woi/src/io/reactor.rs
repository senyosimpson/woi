use std::{cell::RefCell, io, os::unix::prelude::RawFd, rc::Rc, task::Waker, time::Duration};

use slab::Slab;
use once_cell::unsync::Lazy;

use super::epoll::{Epoll, Event, Events, Interest, Token};

#[derive(Clone, Default)]
pub(crate) struct IoSource {
    io: RawFd,
    token: Token,
    tick: usize,
    // TODO: Determine if I really need/want to use interior mutability here
    waiters: [RefCell<Waiter>; 2],
}

#[derive(Clone, Default)]
struct Waiter {
    // waker for poll_readable/poll_writeable
    waker: Option<Waker>,
    // wakers interested in this event
    wakers: Slab<Waker>,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
enum Direction {
    Read,
    Write,
}

impl Waiter {
    fn drain_into(&mut self, vec: &mut Vec<Waker>) {
        if let Some(waker) = self.waker.take() {
            vec.push(waker);
        }

        for waker in self.wakers.drain() {
            vec.push(waker);
        }
    }
}

impl IoSource {
    pub fn wake(&self, event: &Event) {
        let mut wakers = Vec::new();

        if event.is_readable() {
            self.waiters[Direction::Read as usize]
                .borrow_mut()
                .drain_into(&mut wakers);
        }

        if event.is_writeable() {
            self.waiters[Direction::Write as usize]
                .borrow_mut()
                .drain_into(&mut wakers);
        }

        for waker in wakers {
            waker.wake()
        }
    }
}

// Handle to a reactor
pub(crate) struct Handle {
    pub poll: Epoll,
    pub sources: Rc<RefCell<Slab<IoSource>>>,
}

impl Handle {
    pub fn current() {
        
    }
}

pub(crate) struct Reactor {
    poll: Epoll,
    events: Events,
    sources: Rc<RefCell<Slab<IoSource>>>,
}

impl Reactor {
    pub fn new() -> io::Result<Reactor> {
        Ok(Reactor {
            poll: Epoll::new()?,
            events: Events::new(),
            sources: Rc::new(RefCell::new(Slab::new())),
        })
    }

    pub fn handle(&self) -> Handle {
        Handle {
            poll: self.poll.clone(),
            sources: self.sources.clone(),
        }
    }

    pub fn register(&mut self, io: RawFd, interest: Interest) -> io::Result<IoSource> {
        let mut sources = self.sources.borrow_mut();
        let entry = sources.vacant_entry();
        let tick = 0;

        let token = Token(entry.key());
        let waiters = Default::default();
        let io_source = IoSource {
            io,
            token,
            tick,
            waiters,
        };

        // How does the interest get here? I think that'll be defined by the Future
        // implementation of the sockets
        self.poll.add(io, interest, token.clone())?;
        entry.insert(io_source.clone());

        Ok(io_source)
    }

    pub fn deregister(&mut self, source: IoSource) -> io::Result<()> {
        self.sources.borrow_mut().remove(source.token.0);
        self.poll.delete(source.io)
    }

    // Process new events
    pub fn react(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        // TODO: Figure out what the use case for the driver tick is here

        self.poll.poll(&mut self.events, timeout)?;

        for event in self.events.iter() {
            let token = event.token();
            if let Some(io_source) = self.sources.borrow_mut().get(token.0) {
                // TODO: Ensure the resource is not stale
                io_source.wake(event)
            }
        }

        Ok(())
    }
}
