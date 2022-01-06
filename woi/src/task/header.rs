use std::task::Waker;

use crate::task::{raw::TaskVTable, state::State};

pub(crate) struct Header {
    pub state: State,
    pub waker: Option<Waker>,        // Why is this wrapped in UnsafeCell?
    pub vtable: &'static TaskVTable, // Why &'static? Think cause they are fns
}

impl Header {
    pub fn register_waker(&mut self, waker: &Waker) {
        self.waker = Some(waker.clone());
    }

    pub fn wake_join_handle(&self) {
        match &self.waker {
            Some(waker) => waker.wake_by_ref(),
            None => panic!("Missing waker!")
        }
    }
}