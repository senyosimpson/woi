use std::{
    cell::RefCell,
    io,
    os::unix::prelude::RawFd,
    rc::Rc,
    task::{Context, Poll, Waker},
    time::Duration,
};

use slab::Slab;

use super::{epoll::{Epoll, Event, Events, Interest, Token}, readiness::Readiness};

#[derive(Clone, Default)]
pub(crate) struct IoSource {
    readiness: Readiness,
    io: RawFd,
    token: Token,
    tick: usize,
    // TODO: Determine if I want to use interior mutability here
    // Waker registered by poll_readable
    reader: Option<Waker>,
    // TODO: Determine if I want to use interior mutability here
    // Waker registered by poll_writable
    writer: Option<Waker>, // waiters: [RefCell<Waiter>; 2],
}

#[derive(Clone, Default)]
struct Waiter {
    // waker for poll_readable/poll_writeable
    waker: Option<Waker>,
    // TODO: Determine when this is actually necessary
    // For context: I'm only supporting tcp/udp streams. In no
    // circumstance can I think of a way it's possible for multiple
    // tasks to be waiting on the *same* tcp read/write
    // wakers interested in this event
    // wakers: Slab<Waker>,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub(crate) enum Direction {
    Read,
    Write,
}

// impl Waiter {
//     fn drain_into(&mut self, vec: &mut Vec<Waker>) {
//         if let Some(waker) = self.waker.take() {
//             vec.push(waker);
//         }

//         for waker in self.wakers.drain() {
//             vec.push(waker);
//         }
//     }
// }

impl IoSource {
    pub fn set_readiness(&mut self, event: &Event) {
        self.readiness = Readiness::from_event(event)
    }

    pub fn clear_readiness(&mut self, direction: Direction) {
        match direction {
            Direction::Read => self.readiness = self.readiness - Readiness::READABLE,
            Direction::Write => self.readiness = self.readiness - Readiness::WRITABLE
        }
    }


    pub fn wake(&mut self, event: &Event) {
        let mut wakers = Vec::new();

        if event.is_readable() {
            if let Some(waker) = self.reader.take() {
                wakers.push(waker)
            }
        }

        if event.is_writable() {
            if let Some(waker) = self.writer.take() {
                wakers.push(waker)
            }
        }

        for waker in wakers {
            waker.wake()
        }
    }

    pub(crate) fn poll_ready(
        &mut self,
        direction: Direction,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        match direction {
            Direction::Read => {
                if self.readable() {
                    return Poll::Ready(Ok(()))
                }
            },
            Direction::Write => {
                if self.writable() {
                    return Poll::Ready(Ok(()))
                }
            }
        }

        let slot = match direction {
            Direction::Read => &mut self.reader,
            Direction::Write => &mut self.writer,
        };

        match slot {
            Some(existing) => *existing = cx.waker().clone(),
            None => *slot = Some(cx.waker().clone()),
        }

        Poll::Pending
    }

    pub fn poll_readable(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_ready(Direction::Read, cx)
    }

    pub fn poll_writable(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_ready(Direction::Write, cx)
    }

    pub fn readable(&self) -> bool {
        self.readiness & Readiness::READABLE == Readiness::READABLE
    }
    pub fn writable(&self) -> bool {
        self.readiness & Readiness::WRITABLE == Readiness::WRITABLE
    }
}

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
