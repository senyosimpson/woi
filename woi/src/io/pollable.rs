use super::reactor::{Handle, IoSource};

// Async I/O adapter that bridges the event queue and I/O of interest
pub(crate) struct Pollable<T> {
    io: T,
    source: IoSource,
    handle: Handle
}

impl<T> Pollable<T> {
}