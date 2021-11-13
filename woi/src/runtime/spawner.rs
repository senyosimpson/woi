use std::{collections::VecDeque, future::Future, marker::PhantomData, rc::Rc};

use crate::task::{join::JoinHandle, raw::RawTask, task::Task};

pub struct Spawner {
    queue: Rc<VecDeque<Task>>,
}

impl Spawner {
    pub fn spawn<F: Future, S: Schedule>(&self, future: F) -> JoinHandle<F::Output> {
        let raw = RawTask::<_, S>::allocate(future);
        let task = Task { raw };
        let join_handle = JoinHandle {
            raw,
            _marker: PhantomData,
        };

        self.queue.push_back(task);

        join_handle
    }
}
