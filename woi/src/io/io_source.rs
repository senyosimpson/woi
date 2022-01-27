use std::cell::RefCell;
use std::io;
use std::os::unix::prelude::RawFd;
use std::task::{Context, Poll, Waker};

use super::epoll::{Event, Token};
use super::readiness::Readiness;

#[derive(Clone, Default)]
pub(crate) struct IoSource {
    /// Raw file descriptor of the IO resource
    pub(crate) io: RawFd,
    /// Contains the driver tick
    pub(crate) tick: usize,
    /// Token tying io source to slot in reactor slab
    pub(crate) token: Token,
    /// Holds state on an io resource's readiness for
    /// reading and writing 
    pub(crate) inner: RefCell<Inner>
}

#[derive(Clone, Default)]
pub(crate) struct Inner {
    /// Readiness of the source. Used to determine whether
    /// the source is ready for reading/writing
    pub(crate) readiness: Readiness,
    /// Waker registered by poll_readable
    pub(crate) reader: Option<Waker>,
    /// Waker registered by poll_writable
    pub(crate) writer: Option<Waker>,
}

#[derive(Clone, Copy)]
pub(crate) enum Direction {
    Read,
    Write,
}

impl IoSource {
    pub fn set_readiness(&self, event: &Event) {
        let mut inner = self.inner.borrow_mut();
        inner.readiness = Readiness::from_event(event)
    }

    pub fn clear_readiness(&self, direction: Direction) {
        let mut inner = self.inner.borrow_mut();
        match direction {
            Direction::Read => inner.readiness = inner.readiness - Readiness::READABLE,
            Direction::Write => inner.readiness = inner.readiness - Readiness::WRITABLE,
        }
    }

    pub fn wake(&self, event: &Event) {
        let mut wakers = Vec::new();

        let mut inner = self.inner.borrow_mut();

        if event.is_readable() {
            if let Some(waker) = inner.reader.take() {
                wakers.push(waker)
            }
        }

        if event.is_writable() {
            if let Some(waker) = inner.writer.take() {
                wakers.push(waker)
            }
        }

        for waker in wakers {
            waker.wake()
        }
    }

    pub(crate) fn poll_ready(
        &self,
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

        let mut inner = self.inner.borrow_mut();

        let slot = match direction {
            Direction::Read => &mut inner.reader,
            Direction::Write => &mut inner.writer,
        };

        match slot {
            Some(existing) => *existing = cx.waker().clone(),
            None => *slot = Some(cx.waker().clone()),
        }

        Poll::Pending
    }

    pub fn poll_readable(&self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        tracing::debug!("Invoking poll_readable");
        let res = self.poll_ready(Direction::Read, cx);
        match res {
            Poll::Ready(Ok(())) => tracing::debug!("poll_readable returned Poll::Ready(ok)"),
            Poll::Ready(Err(_)) => tracing::debug!("poll_readable returned Poll::Ready(err)"),
            Poll::Pending => tracing::debug!("poll_readable returned Poll::Pending")
        }
        res
    }

    pub fn poll_writable(&self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_ready(Direction::Write, cx)
    }

    pub fn readable(&self) -> bool {
        let inner = self.inner.borrow();
        inner.readiness & Readiness::READABLE == Readiness::READABLE
    }
    pub fn writable(&self) -> bool {
        let inner = self.inner.borrow();
        inner.readiness & Readiness::WRITABLE == Readiness::WRITABLE
    }
}
