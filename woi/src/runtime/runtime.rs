use std::{cell::RefCell, collections::VecDeque, future::Future, rc::Rc};
// use slab::Slab;
// use crate::task::{self, Task, JoinHandle};
use crate::runtime::spawner::Spawner;
use crate::task::task::Task;

pub struct Runtime {
    // Queue that holds tasks that are ready to be executed
    queue: Rc<VecDeque<Task>>,
    // Spawner responsible for spawning tasks onto the executor
    spawner: Spawner
}

impl Runtime {
    pub fn new() -> Runtime {
        let queue = Rc::new(VecDeque::new());
        Runtime {
            queue,
            spawner: Spawner { queue }
        }
    }

    pub fn spawner() -> Spawner {
        spawner
    }

    pub fn run(&self) {
        loop {
            if let Some(task) = self.queue.borrow_mut().pop_front() {
                println!("GOT TASK");
                // task.future.as_mut().poll(cx)
            }
        }
    }

    // pub fn spawn<F>(&self, future: F) -> Task<F::Output>
    // where
    //     F: Future + 'static,
    // {
    //     // let queue = self.queue.clone();
    //     // let schedule_fn = |schedulable| queue.push_back(schedulable);
    //     task::spawn(future, self.queue.clone())
    // }
}
