use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::marker::PhantomData;
use std::rc::Rc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use super::context;
use crate::io::reactor::{Handle as IoHandle, Reactor};
use crate::task::join::JoinHandle;
use crate::task::raw::{RawTask, Schedule};
use crate::task::Task;

pub struct Runtime {
    // Holds the reactor and task queue
    inner: RefCell<Inner>,
    // Handle to runtime
    handle: Handle,
}

struct Inner {
    /// IO reactor
    reactor: Reactor,
    /// Queue that holds tasks
    queue: Queue,
}

/// Handle to the runtime
#[derive(Clone)]
pub struct Handle {
    /// Spawner responsible for spawning tasks onto the executor
    pub(crate) spawner: Spawner,
    /// Handle to the IO reactor
    pub(crate) io: IoHandle,
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
            spawner,
            io: io_handle,
        };

        let inner = RefCell::new(Inner { reactor, queue });

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
        // Enter runtime context
        let _enter = context::enter(self.handle.clone());
        self.inner.borrow_mut().block_on(future)
    }
}

// ===== impl Inner =====

impl Inner {
    pub fn block_on<F: Future>(&mut self, future: F) -> F::Output {
        crate::pin!(future);

        let waker = unsafe { Waker::from_raw(NoopWaker::waker()) };
        let cx = &mut Context::from_waker(&waker);

        loop {
            // If the future is ready, return the output
            tracing::debug!("Polling `block_on` future");
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
            loop {
                let task = self.queue.borrow_mut().pop_front();
                match task {
                    Some(task) => {
                        tracing::debug!(
                            "Task {}: Popped off executor queue and running",
                            task.id()
                        );
                        task.run()
                    }
                    None => break,
                }
            }
        }
    }
}

// ===== impl Handle =====

impl Handle {
    pub fn spawn<F: Future>(&self, future: F) -> JoinHandle<F::Output> {
        self.spawner.spawn(future)
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
        tracing::debug!("Task {}: Spawned", task.id());

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

// ===== No op waker =====

struct NoopWaker;

impl NoopWaker {
    fn waker() -> RawWaker {
        fn no_op(_: *const ()) {}
        fn clone(_: *const ()) -> RawWaker {
            NoopWaker::waker()
        }

        let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
        RawWaker::new(0 as *const (), vtable)
    }
}
