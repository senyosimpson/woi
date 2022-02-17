use std::cell::RefCell;
use std::future::Future;

use super::runtime::Spawner;
use crate::io::reactor::Handle as IoHandle;
use crate::task::JoinHandle;

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
    pub fn spawn<F: Future>(&self, future: F) -> JoinHandle<F::Output> {
        self.spawner.spawn(future)
    }
}