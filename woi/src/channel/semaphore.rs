use core::cell::{Cell, RefCell};
use core::future::Future;
use core::pin::Pin;
use core::ptr;
use core::task::{Context, Poll, Waker};

use super::linked_list::LinkedList;

pub struct Semaphore {
    permits: Cell<u32>,
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

pub struct Permit {}

impl Semaphore {
    pub fn new(permits: u32) -> Semaphore {
        Semaphore {
            permits: Cell::new(permits),
            waiters: RefCell::new(LinkedList::new()),
        }
    }

    /// Acquire a permit that gives access to the data
    ///
    /// If there are no permits left, a waker gets put into the semaphore
    /// waitlist and we wait until one is available
    pub fn poll_acquire(
        &self,
        cx: &mut Context,
        waiter: &mut Waiter,
    ) -> Poll<Result<(), AcquireError>> {
        let permits = self.permits.get();
        if permits > 0 {
            self.permits.set(permits - 1);
            return Poll::Ready(Ok(()));
        }

        waiter.waker = Some(cx.waker().clone());
        let waiter_ptr = waiter as *const _ as *mut Waiter;
        self.waiters.borrow_mut().push_back(waiter_ptr);

        Poll::Pending
    }

    pub fn acquire(&self) -> Acquire<'_> {
        Acquire::new(self)
    }
}

impl Waiter {
    pub fn new() -> Waiter {
        Waiter {
            waker: None,
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
        }
    }
}

impl<'a> Acquire<'a> {
    fn new(semaphore: &'a Semaphore) -> Acquire {
        Acquire { semaphore, waiter: Waiter::new() }
    }
}

impl Future for Acquire<'_> {
    type Output = Result<(), AcquireError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        this.semaphore.poll_acquire(cx, &mut this.waiter)
    }
}
