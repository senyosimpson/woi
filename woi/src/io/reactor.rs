use std::cell::RefCell;
use std::io;
use std::os::unix::prelude::RawFd;
use std::rc::Rc;
use std::time::Duration;

use slab::Slab;

use super::epoll::{Epoll, Events, Interest, Token};
use super::io_source::IoSource;

pub(crate) struct Inner {
    pub poll: RefCell<Epoll>,
    // TODO: This sucks lmao
    pub sources: RefCell<Slab<Rc<RefCell<IoSource>>>>,
}

#[derive(Clone)]
pub(crate) struct Handle {
    pub inner: Rc<Inner>,
}

impl Handle {
    pub fn current() -> Self {
        crate::runtime::handle::io()
    }
}

pub(crate) struct Reactor {
    events: Events,
    inner: Rc<Inner>,
}

impl Reactor {
    pub fn new() -> io::Result<Reactor> {
        Ok(Reactor {
            events: Events::new(),
            inner: Rc::new(Inner {
                poll: RefCell::new(Epoll::new()?),
                sources: RefCell::new(Slab::new()),
            }),
        })
    }

    pub fn handle(&self) -> Handle {
        Handle {
            inner: self.inner.clone(),
        }
    }

    // Process new events
    pub fn react(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        // TODO: Figure out what the use case for the driver tick is here

        // TODO: Simply construction of types to remove this long call
        self.inner
            .poll
            .borrow_mut()
            .poll(&mut self.events, timeout)?;

        for event in self.events.iter() {
            let token = event.token();
            if let Some(io_source) = self.inner.sources.borrow_mut().get_mut(token.0) {
                // TODO: Ensure the resource is not stale
                io_source.borrow_mut().set_readiness(event);
                io_source.borrow_mut().wake(event)
            }
        }

        Ok(())
    }
}

// NOTE: Attaching these methods to the handle as a hack. There should be some shared
// construct between the handle and the reactor for registering sources
impl Handle {
    pub fn register(&mut self, io: RawFd, interest: Interest) -> io::Result<Rc<RefCell<IoSource>>> {
        let mut sources = self.inner.sources.borrow_mut();
        let entry = sources.vacant_entry();
        let tick = 0;

        let token = Token(entry.key());
        let io_source = Rc::new(RefCell::new(IoSource {
            io,
            token,
            tick,
            ..Default::default()
        }));

        self.inner
            .poll
            .borrow_mut()
            .add(io, interest, token.clone())?;

        entry.insert(io_source.clone());

        Ok(io_source)
    }

    pub fn deregister(&mut self, source: IoSource) -> io::Result<()> {
        self.inner.sources.borrow_mut().remove(source.token.0);
        self.inner.poll.borrow_mut().delete(source.io)
    }
}
