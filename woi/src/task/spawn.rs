use std::future::Future;
use crate::task::join::JoinHandle;

// pub fn spawn<F: Future>(future: F) -> JoinHandle<F::Output> {
//     crate::runtime::Runtime::spawner().spawn(future)
// }