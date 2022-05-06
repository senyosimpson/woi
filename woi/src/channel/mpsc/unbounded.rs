//! An unbounded multi-producer, single-consumer queue for sending values between
//! asynchronous tasks.

use std::rc::Rc;

use futures::future::poll_fn;

use super::channel::Channel;
use crate::channel::error::{SendError, TryRecvError};

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let chan = Rc::new(Channel::new(128));
    (Sender::new(chan.clone()), Receiver::new(chan))
}

pub struct Sender<T> {
    chan: Rc<Channel<T>>,
}

pub struct Receiver<T> {
    chan: Rc<Channel<T>>,
}

// ==== impl Sender =====

impl<T> Sender<T> {
    pub fn new(chan: Rc<Channel<T>>) -> Sender<T> {
        Sender { chan }
    }

    // This does not need to be async as sending to an unbounded queue
    // will never block
    pub fn send(&self, message: T) -> Result<(), SendError<T>> {
        self.chan.send(message)
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.chan.incr_tx_count();
        Self {
            chan: self.chan.clone(),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        tracing::debug!("Dropping sender");
        self.chan.decr_tx_count();
        if self.chan.tx_count() == 0 {
            self.chan.close();
        }
    }
}

// ===== impl Receiver =====

impl<T> Receiver<T> {
    pub fn new(chan: Rc<Channel<T>>) -> Receiver<T> {
        Receiver { chan }
    }

    pub async fn recv(&self) -> Option<T> {
        poll_fn(|cx| self.chan.recv(cx)).await
    }

    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.chan.try_recv()
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        tracing::debug!("Dropping receiver");
        self.chan.close();
    }
}