use std::cell::RefCell;

use super::handle::Handle;
use super::runtime::Spawner;
use crate::io::reactor::Handle as IoHandle;


thread_local! {
    static CONTEXT: RefCell<Option<Handle>> = RefCell::new(None)
}

pub(crate) fn enter(new: Handle) -> EnterGuard {
    match CONTEXT.try_with(|ctx| {
        let old = ctx.borrow_mut().replace(new);
        EnterGuard(old)
    }) {
        Ok(enter_guard) => enter_guard,
        Err(_) => panic!("Thread local destroyed")
    }
}

pub(crate) struct EnterGuard(Option<Handle>);
// pub(crate) struct EnterGuard();

impl Drop for EnterGuard {
    fn drop(&mut self) {
        tracing::debug!("Dropping enter guard");
        CONTEXT.with(|ctx| {
            // *ctx.borrow_mut() = self.0.take();
            ctx.borrow_mut().take();
            // self.0.take();
        })
    }
}

// API wise this isn't that sound versus having context::io(). However
// I've just put it here for the sake of simplicity. If there arises a
// need to change this, I will.
pub(crate) fn io() -> IoHandle {
    match CONTEXT.try_with(|ctx| {
        let ctx = ctx.borrow();
        let handle = ctx.as_ref().expect("No reactor running");
        handle.io.clone()
    }) {
        Ok(io_handle) => io_handle,
        Err(_) => panic!("Thread local destroyed"),
    }
}

pub(crate) fn spawner() -> Spawner {
    match CONTEXT.try_with(|ctx| {
        let ctx = ctx.borrow();
        ctx.as_ref()
            .map(|handle| handle.spawner.clone())
            .expect("No reactor running")
    }) {
        Ok(spawner) => spawner,
        Err(_) => panic!("Thread local destroyed"),
    }
}