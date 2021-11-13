use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};

use crate::task::{header::Header, raw::RawTask};

pub struct JoinHandle<T> {
    // Pointer to raw task
    raw: NonNull<()>,
    _marker: PhantomData<T>,
}

impl<T> JoinHandle<T> {
    pub fn new<F: Future>(future: F) -> JoinHandle<T> {
        let ptr = RawTask::<_, >::allocate(future);
        JoinHandle {
            raw: ptr,
            _marker: PhantomData,
        }
    }

    pub fn poll_inner() {}
}

impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let raw = self.raw.as_ptr();
        let header = raw as *const Header;

        // This is obviously not sane code. There needs to be checks to see if we
        // can actually read the output
        unsafe {
            let output = {
                let out = ((*header).vtable.get_output)(self.raw.as_ptr());
                (out as *mut T).read()
            };

            Poll::Ready(output)
        }
    }
}
