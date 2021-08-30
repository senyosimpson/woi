use std::{boxed::Box, cell::RefCell, collections::VecDeque, future::Future, pin::Pin, rc::Rc, sync::Arc, task::Context};

use futures::task::{self, ArcWake};

#[derive(PartialEq, Eq)]
pub struct Token(pub usize);

pub struct Shared {}

pub struct Schedulable {
    shared: Rc<Shared>,
}

type JoinHandle<T> = Task<T>;

pub struct Task<T> {
    future: Pin<Box<dyn Future<Output = T>>>,
    inner: Rc<Shared>,
}

// pub struct Task<T> {
//     future: Mutex<Pin<Box<dyn Future<Output = T>>>>,
//     executor: channel::Sender<Arc<Task<T>>>,
// }

// T is being enforced to be Send. This is in order
// to uphold the guarantee that it is both Send + Sync. If T is
// not Send, then it can't be Send and Sync and all type checks
// will fail.
// I'm not sure I need that for Sync but let's just leave it
// Got the idea from channel.rs in Crossbeam crate
// unsafe impl<T: Send> Send for Task<T>;
// unsafe impl<T> Sync for Task<T>;

// Not adding a bound here. I'm not sure if that makes sense but
// let's see how far we get
impl<T> Task<T> {
//     fn schedule(self: &Arc<Self>) {
// self.executor.send(self.clone()).unwrap();
// }

// pub(crate) fn poll(self: Arc<Self>) -> T {
//     let waker = task::waker(self.clone());
//     let mut cx = Context::from_waker(&waker);

//     let mut future = self.future.try_lock().unwrap();
//     future.as_mut().poll(&mut cx)
// }

// pub(crate) fn spawn<F>(future: F, sender: &channel::Sender<Arc<Task<T>>>)
// where
//     F: Future<Output = T> + 'static,
// {
//     let task = Arc::new(Task {
//         future: Mutex::new(Box::pin(future)),
//         executor: sender.clone(),
//     });

//     sender.send(task).unwrap();
// }
// }

// impl<T> ArcWake for Task<T> {
//     fn wake(self: Arc<Self>) {
//         self.schedule();
//     }

//     fn wake_by_ref(arc_self: &Arc<Self>) {
//         arc_self.schedule();
//     }
}

pub fn spawn<F>(future: F, queue: Rc<RefCell<VecDeque<Schedulable>>>) -> Task<F::Output>
where
    F: Future + 'static,
    F::Output: 'static,
{
    let shared = Rc::new(Shared {});
    let task = Task {
        future: Box::pin(future),
        inner: shared.clone()
    };

    let schedulable = Schedulable {
        shared: shared.clone()
    };
    queue.borrow_mut().push_back(schedulable);

    task
}
