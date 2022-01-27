use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::marker::PhantomData;
use std::rc::Rc;
use std::task::{Context, Poll};

use super::handle::Handle;
use crate::io::reactor::Reactor;
use crate::task::join::JoinHandle;
use crate::task::raw::{RawTask, Schedule};
use crate::task::task::Task;

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

#[derive(Clone)]
pub struct Spawner {
    queue: Queue,
}

type Queue = Rc<RefCell<VecDeque<Task>>>;

// ===== impl Runtime =====

impl Runtime {
    pub fn new() -> Runtime {
        let queue = Rc::new(RefCell::new(VecDeque::new()));
        let spawner = Spawner {
            queue: queue.clone(),
        };

        let reactor = Reactor::new().expect("Could not start reactor!");
        let io_handle = reactor.handle();

        // Runtime handle
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

            // Since we're here, we know the 'block_on' future isn't ready. We then
            // check if there have been tasks scheduled onto the runtime.
            // 1. If there are no tasks on the runtime, it means we're waiting on IO
            //    resources (e.g I'm performing a read and waiting on data to arrive).
            //    Essentially, this means we have events registered in our reactor and
            //    we are waiting for them to fire.
            // 2. If there are tasks spawned onto the runtime, we can start processing them
            if self.queue.borrow().is_empty() {
                tracing::debug!("Parking on epoll");
                self.reactor
                    .react(None)
                    .expect("Reactor failed to process events");
            }

            // We have tasks to process. We process all of them. After, we proceed to
            // to poll the outer future again with the hope that we aren't waiting on
            // anymore resources and are now finished our work (unless we are a web
            // server of course)
            while let Some(task) = self.queue.borrow_mut().pop_front() {
                task.run();
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
