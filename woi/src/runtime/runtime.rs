use std::task::Context;
use std::{
    cell::RefCell, collections::VecDeque, future::Future, marker::PhantomData, rc::Rc,
    task::Poll,
};

use crate::task::raw::Schedule;
use crate::task::{join::JoinHandle, raw::RawTask, task::Task};

type Queue = Rc<RefCell<VecDeque<Task>>>;

impl Schedule for Queue {
    fn schedule(&self, task: Task) {
        // TODO: Handle unwrapping
        self.borrow_mut().push_back(task);
    }
}

pub struct Spawner {
    queue: Queue,
}

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

// Initialise only Runtime
// use once_cell::unsync::Lazy;
// static RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new());

pub struct Runtime {
    // Queue that holds tasks that are ready to be executed
    queue: Queue,
    // Spawner responsible for spawning tasks onto the executor
    spawner: Spawner,
}

impl Runtime {
    pub fn new() -> Runtime {
        let queue = Rc::new(RefCell::new(VecDeque::new()));
        let spawner = Spawner {
            queue: queue.clone(),
        };

        Runtime { queue, spawner }
    }

    pub fn spawner(&self) -> &Spawner {
        &self.spawner
    }

    pub fn spawn<F: Future>(&self, future: F) -> JoinHandle<F::Output> {
        self.spawner.spawn(future)
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        use std::task::Waker;

        // let future = Pin::new_unchecked(future);

        let mut future = Box::pin(future);
        let waker = unsafe { Waker::from_raw(dummy_raw_waker()) };
        let cx = &mut Context::from_waker(&waker);
        // // somehow get a waker and context
        loop {
            match future.as_mut().poll(cx) {
                Poll::Ready(output) => return output,
                // Just let it busy loop for now
                Poll::Pending => {
                    // Go through all elements in the queue
                    // When all have been processed, poll's the outer future again
                    if let Some(task) = self.queue.borrow_mut().pop_front() {
                        task.poll();
                    }
                }
            }
        }
    }
}

// Dummy raw waker for now

use core::task::RawWaker;
use core::task::RawWakerVTable;

fn dummy_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(0 as *const (), vtable)
}
