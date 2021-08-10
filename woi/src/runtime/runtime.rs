use crossbeam::channel;
use std::{future::Future, sync::Arc};
// use slab::Slab;

use crate::task::Task;

pub struct Runtime {
    /// Queue that holds tasks that are ready to be executed
    scheduled: channel::Receiver<Arc<Task>>,
    /// Sends tasks to the `scheduled` queue. This is passed to
    /// tasks so that they're able to schedule themselves for
    /// execution by the executor
    sender: channel::Sender<Arc<Task>>,
}

impl Runtime {
    pub fn new() -> Runtime {
        let (sender, scheduled) = channel::unbounded();
        Runtime { scheduled, sender }
    }

    pub fn block_on(&self) {
        while let Ok(task) = self.scheduled.recv() {
            task.poll();
        }
    }

    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Task::spawn(future, &self.sender)
    }
}
