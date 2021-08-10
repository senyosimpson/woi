use std::{
    boxed::Box,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use crossbeam::channel;
use futures::task::{self, ArcWake};

#[derive(PartialEq, Eq)]
pub struct Token(pub usize);

// Task should have wakers and handle to queue
pub struct Task {
    // Do not need the mutex here since it is a single threaded executor
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
    executor: channel::Sender<Arc<Task>>,
}

impl Task {
    fn schedule(self: &Arc<Self>) {
        self.executor.send(self.clone()).unwrap();
    }

    pub(crate) fn poll(self: Arc<Self>) {
        let waker = task::waker(self.clone());
        let mut cx = Context::from_waker(&waker);

        let mut future = self.future.try_lock().unwrap();
        future.as_mut().poll(&mut cx);
    }

    pub(crate) fn spawn<F>(future: F, sender: &channel::Sender<Arc<Task>>)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
            executor: sender.clone(),
        });

        sender.send(task).unwrap();
    }
}

impl ArcWake for Task {
    fn wake(self: Arc<Self>) {
        self.schedule();
    }

    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.schedule();
    }
}
