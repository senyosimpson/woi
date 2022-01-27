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
        // Should I use interior mutability here instead?
        let header = raw as *mut Header;

        unsafe {
            let state = &(*header).state;
            if !state.is_complete() {
                // Register waker with the task
                (*header).register_waker(cx.waker());
                (*header).state.set_join_waker();
            } else {
                let output = {
                    let out = ((*header).vtable.get_output)(self.raw.as_ptr());
                    (out as *mut T).read()
                };
                return Poll::Ready(output);
            }

            return Poll::Pending;
        }
    }
}
