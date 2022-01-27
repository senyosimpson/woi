use std::cell::RefCell;
use std::future::Future;

use tracing;

use super::runtime::Spawner;
use crate::io::reactor::Handle as IoHandle;
use crate::task::join::JoinHandle;

thread_local! {
    static CONTEXT: RefCell<Option<Handle>> = RefCell::new(None)
}

// Handle to the runtime
#[derive(Clone)]
pub struct Handle {
    // Spawner responsible for spawning tasks onto the executor
    pub(crate) spawner: Spawner,
    // Handle to the IO reactor
    pub(crate) io: IoHandle,
}

impl Handle {
    // Store the handle in the runtime context
    pub(crate) fn register(&self) {
        CONTEXT.with(|ctx| {
            *ctx.borrow_mut() = Some(self.clone());
        })
    }

    pub fn spawn<F: Future>(&self, future: F) -> JoinHandle<F::Output> {
        self.spawner.spawn(future)
    }
}

// API wise this isn't that sound versus having context::io(). However
// I've just put it here for the sake of simplicity. If there arises a
// need to change this, I will.
pub(crate) fn io() -> IoHandle {
    match CONTEXT.with(|ctx| ctx.borrow().clone()) {
        Some(handle) => handle.io.clone(),
        None => panic!("No io runtime handle fool!"),
    }
}
