use std::{
    cell::RefCell,
    io,
    os::unix::prelude::RawFd,
    rc::Rc,
    time::Duration,
};

use slab::Slab;

use super::epoll::{Epoll, Events, Interest, Token};
use super::io_source::IoSource;


// Handle to the reactor
#[derive(Clone)]
pub(crate) struct Handle {
    pub poll: Epoll,
    pub sources: Rc<RefCell<Slab<IoSource>>>,
}

impl Handle {
    pub fn current() -> Self {
        crate::runtime::handle::io()
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

    // Process new events
    pub fn react(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        // TODO: Figure out what the use case for the driver tick is here

        self.poll.poll(&mut self.events, timeout)?;

        for event in self.events.iter() {
            let token = event.token();
            if let Some(io_source) = self.sources.borrow_mut().get_mut(token.0) {
                // TODO: Ensure the resource is not stale
                io_source.set_readiness(event);
                io_source.wake(event)
            }
        }

        Ok(())
    }
}

// NOTE: Attaching these methods to the handle as a hack. There should be some shared
// construct between the handle and the reactor for registering sources
impl Handle {
    pub fn register(&mut self, io: RawFd, interest: Interest) -> io::Result<IoSource> {
        let mut sources = self.sources.borrow_mut();
        let entry = sources.vacant_entry();
        let tick = 0;

        let token = Token(entry.key());
        let io_source = IoSource {
            io,
            token,
            tick,
            ..Default::default()
        };

        self.poll.add(io, interest, token.clone())?;
        entry.insert(io_source.clone());

        Ok(io_source)
    }

    pub fn deregister(&mut self, source: IoSource) -> io::Result<()> {
        self.sources.borrow_mut().remove(source.token.0);
        self.poll.delete(source.io)
    }
}
