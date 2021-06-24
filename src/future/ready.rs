use std::{future::Future, pin::Pin, task::{Context, Poll}};

pub struct Ready<T>(Option<T>);

// At some point, I should probably write a post on this! Unpin is used to *mark* a type
// as movable - i.e that it can be moved without issue. Essentially it nullifies the
// effect of Pin on the type. Unpin is marked for almost all standard types in the Rust
// lib. It is also an auto-trait.
// More here: https://doc.rust-lang.org/reference/special-types-and-traits.html#auto-traits
// Considering we are working over some generic type T, we need to explicitly set this.
// An alternative way of doing this would be to implement Future on Ready for all types T
// that implement Unpin using a trait bound. This way of doing it is preferable, imo, since
// it is more explicit and will show up in the documentation
impl<T> Unpin for Ready<T> {}

impl<T> Future for Ready<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.0.take().unwrap())
    }
}

pub fn ok<T, E>(t: T) -> Ready<Result<T, E>> {
    Ready(Some(Ok(t)))
}