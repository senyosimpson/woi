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
    pub(crate) raw: NonNull<()>,
    pub(crate) _marker: PhantomData<T>,
}

impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        use crate::task::state::Status;

        let raw = self.raw.as_ptr();
        let header = raw as *const Header;

        unsafe {
            let status = &(*header).state.status;
            match status {
                Status::Done => {
                    let output = {
                        let out = ((*header).vtable.get_output)(self.raw.as_ptr());
                        (out as *mut T).read()
                    };

                    return Poll::Ready(output);
                }
                _ => return Poll::Pending,
            }
        }
    }
}
