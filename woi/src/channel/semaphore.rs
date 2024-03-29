use core::cell::{Cell, RefCell};
use core::future::Future;
use core::pin::Pin;
use core::ptr;
use core::task::{Context, Poll, Waker};

use super::linked_list::LinkedList;

pub struct Semaphore {
    permits: Cell<usize>,
    waiters: RefCell<LinkedList>,
}

// We have to make this unpin so that we ensure that the waiter isn't
// moved in memory. I need to read up on this though
pub struct Waiter {
    pub(crate) waker: Option<Waker>,
    pub(crate) next: *mut Waiter,
    pub(crate) prev: *mut Waiter,
}

/// Future to acquire a permit for sending messages to the channel
pub struct Acquire<'a> {
    semaphore: &'a Semaphore,
    waiter: Waiter,
}

pub struct AcquireError;

// ===== impl Semaphore =====

impl Semaphore {
    pub fn new(permits: usize) -> Semaphore {
        Semaphore {
            permits: Cell::new(permits),
            waiters: RefCell::new(LinkedList::new()),
        }
    }

    pub(crate) fn release(&self) {
        self.permits.set(self.permits.get() + 1);
        tracing::debug!("Released permit. Available: {}", self.permits.get());

        let mut waiters = self.waiters.borrow_mut();
        if let Some(waiter) = waiters.pop_front() {
            // TODO: Drop the waker?
            if let Some(waker) = &waiter.waker {
                waker.wake_by_ref()
            }
        }
    }

    /// Acquire a permit that gives access to the data
    ///
    /// If there are no permits left, a waker gets put into the semaphore
    /// waitlist and we wait until one is available
    pub(crate) fn poll_acquire(
        &self,
        cx: &mut Context,
        waiter: &mut Waiter,
    ) -> Poll<Result<(), AcquireError>> {
        let permits = self.permits.get();
        if permits > 0 {
            self.permits.set(permits - 1);
            tracing::debug!("Acquired permit. Available: {}", self.permits.get());
            return Poll::Ready(Ok(()));
        }

        tracing::debug!("No permits available!");
        waiter.waker = Some(cx.waker().clone());
        let waiter_ptr = waiter as *const _ as *mut Waiter;
        self.waiters.borrow_mut().push_back(waiter_ptr);

        Poll::Pending
    }

    pub fn acquire(&self) -> Acquire<'_> {
        Acquire::new(self)
    }
}

// ===== impl Waiter =====

impl Waiter {
    pub fn new() -> Waiter {
        Waiter {
            waker: None,
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
        }
    }
}

// ===== impl Acquire =====

impl<'a> Acquire<'a> {
    pub fn new(semaphore: &'a Semaphore) -> Acquire {
        Acquire {
            semaphore,
            waiter: Waiter::new(),
        }
    }
}

impl Future for Acquire<'_> {
    type Output = Result<(), AcquireError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        this.semaphore.poll_acquire(cx, &mut this.waiter)
    }
}
