use std::io;
use std::os::unix::prelude::RawFd;
use std::task::{Context, Poll, Waker};

use super::epoll::{Event, Token};
use super::readiness::Readiness;

#[derive(Clone, Default)]
pub(crate) struct IoSource {
    // Raw file descriptor of the IO resource
    pub(crate) io: RawFd,
    // Contains the driver tick
    pub(crate) tick: usize,
    // Token tying io source to slot in reactor slab
    pub(crate) token: Token,
    // Readiness of the source. Used to determine whether
    // the source is ready for reading/writing
    pub(crate) readiness: Readiness,
    // Waker registered by poll_readable
    pub(crate) reader: Option<Waker>,
    // Waker registered by poll_writable
    pub(crate) writer: Option<Waker>,
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

#[derive(Clone, Copy)]
pub(crate) enum Direction {
    Read,
    Write,
}

impl IoSource {
    pub fn set_readiness(&mut self, event: &Event) {
        self.readiness = Readiness::from_event(event)
    }

    pub fn clear_readiness(&mut self, direction: Direction) {
        match direction {
            Direction::Read => self.readiness = self.readiness - Readiness::READABLE,
            Direction::Write => self.readiness = self.readiness - Readiness::WRITABLE,
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
                    return Poll::Ready(Ok(()));
                }
            }
            Direction::Write => {
                if self.writable() {
                    return Poll::Ready(Ok(()));
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
