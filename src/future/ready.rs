use std::{future::Future, pin::Pin, task::{Context, Poll}};

// A future that is immediately ready
pub struct Ready<T>(pub Option<T>);

// 
impl<T> Unpin for Ready<T> {}

impl<T> Future for Ready<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.0.take().unwrap())
    }
}
