use std::sync::atomic::AtomicUsize;

use crate::task::raw::TaskVTable;



pub(crate) struct Header {
    pub state: AtomicUsize,
    pub vtable: &'static TaskVTable, // Why &'static? Think cause they are fns
}
