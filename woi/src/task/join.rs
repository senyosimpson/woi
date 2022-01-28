use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::ptr::NonNull;
use std::task::{Context, Poll};

use crate::task::header::Header;

/// A handle to the task
pub struct JoinHandle<T> {
    /// Pointer to raw task
    pub(crate) raw: NonNull<()>,
    pub(crate) _marker: PhantomData<T>,
}

impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let raw = self.raw.as_ptr();
        let header = raw as *mut Header;
        let mut output = Poll::Pending;

        unsafe {
            tracing::debug!("JoinHandle is complete: {}", (*header).state.is_complete());

            if !(*header).state.is_complete() {
                // Register waker with the task
                (*header).register_waker(cx.waker());
                (*header).state.set_join_waker();
            } else {
                tracing::debug!("JoinHandle ready");
                ((*header).vtable.get_output)(self.raw.as_ptr(), &mut output as *mut _ as *mut ());
                return output;
            }

            return output;
        }
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        tracing::debug!("Dropping JoinHandle");
        let raw = self.raw.as_ptr();
        let header = raw as *mut Header;
        unsafe { ((*header).vtable.drop_join_handle)(self.raw.as_ptr()) }
    }
}
