use std::{
    cell::RefCell,
    collections::VecDeque,
    future::Future,
    marker::PhantomData,
    rc::Rc,
    task::{Context, Poll},
};

use super::handle::Handle;
use crate::{
    io::reactor::Reactor,
    task::{join::JoinHandle, raw::RawTask, raw::Schedule, task::Task},
};

#[derive(Clone)]
pub struct Spawner {
    queue: Queue,
}

type Queue = Rc<RefCell<VecDeque<Task>>>;

pub struct Runtime {
    // Holds the reactor and task queue
    inner: RefCell<Inner>,
    // Handle to runtime
    handle: Handle,
}

pub struct Inner {
    // IO reactor
    reactor: Reactor,
    // Queue that holds tasks
    queue: Queue,
}

// ===== impl Runtime =====

impl Runtime {
    pub fn new() -> Runtime {
        let queue = Rc::new(RefCell::new(VecDeque::new()));
        let spawner = Spawner {
            queue: queue.clone(),
        };

        let reactor = Reactor::new().expect("Could not start reactor!");
        let io_handle = reactor.handle();

        // runtime handle
        let handle = Handle {
            spawner: spawner.clone(),
            io: io_handle,
        };

        let inner = RefCell::new(Inner { reactor, queue });

        // Store handle in context
        handle.register();

        Runtime { inner, handle }
    }

    // Get the handle to the runtime
    pub fn handle(&self) -> &Handle {
        &self.handle
    }

    // Spawn a task onto the runtime
    pub fn spawn<F: Future>(&self, future: F) -> JoinHandle<F::Output> {
        self.handle.spawn(future)
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.inner.borrow_mut().block_on(future)
    }
}

// ===== impl Inner =====

impl Inner {
    pub fn block_on<F: Future>(&mut self, future: F) -> F::Output {
        use std::task::Waker;

        crate::pin!(future);

        let waker = unsafe { Waker::from_raw(dummy_raw_waker()) };
        let cx = &mut Context::from_waker(&waker);

        loop {
            // If the future is ready, return the output
            if let Poll::Ready(v) = future.as_mut().poll(cx) {
                return v;
            }

            // Since we're here, we know the 'block_on' future isn't ready. At the same time,
            // there are no events to process meaning that we are waiting for some
            // tasks. We "park" the thread by waiting on the reactor for new events
            if self.queue.borrow().is_empty() {
                self.reactor
                    .react(None)
                    .expect("Reactor failed to process events");
            }

            // We have events to process. We process all of them and then proceed
            // to poll the outer future again.
            while let Some(task) = self.queue.borrow_mut().pop_front() {
                task.poll();
            }
        }
    }
}

// ===== impl Spawner =====

impl Spawner {
    pub fn spawn<F: Future>(&self, future: F) -> JoinHandle<F::Output> {
        let raw = RawTask::new(future, self.queue.clone());
        let task = Task { raw };
        let join_handle = JoinHandle {
            raw,
            _marker: PhantomData,
        };

        self.queue.schedule(task);

        join_handle
    }
}

// ===== impl Queue =====

impl Schedule for Queue {
    fn schedule(&self, task: Task) {
        self.borrow_mut().push_back(task);
    }
}

// Dummy raw waker for now

use core::task::RawWaker;
use core::task::RawWakerVTable;

fn dummy_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn wake(_: *const ()) {
        println!("WOKEN!");
    }
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, wake, wake, no_op);
    RawWaker::new(0 as *const (), vtable)
}
