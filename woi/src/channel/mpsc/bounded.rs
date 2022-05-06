//! A bounded multi-producer, single-consumer queue for sending values between
//! asynchronous tasks.

use std::rc::Rc;

use futures::future::poll_fn;

use super::channel::Channel;
use crate::channel::error::{SendError, TryRecvError};

pub fn channel<T>(size: usize) -> (Sender<T>, Receiver<T>) {
    let chan = Rc::new(Channel::new(size));
    (Sender::new(chan.clone()), Receiver::new(chan))
}

pub struct Permit<T> {
    chan: Rc<Channel<T>>,
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

    pub async fn send(&self, message: T) -> Result<(), SendError<T>> {
        match self.reserve().await {
            Ok(permit) => permit.send(message),
            Err(_) => Err(SendError(message)),
        }
    }

    pub async fn reserve(&self) -> Result<Permit<T>, SendError<()>> {
        match self.chan.semaphore().acquire().await {
            Ok(_) => {
                let permit = Permit {
                    chan: self.chan.clone(),
                };
                Ok(permit)
            }
            Err(_) => Err(SendError(())),
        }
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

// ===== impl Permit =====

impl<T> Permit<T> {
    pub fn send(&self, message: T) -> Result<(), SendError<T>> {
        self.chan.send(message)
    }
}

impl<T> Drop for Permit<T> {
    fn drop(&mut self) {
        self.chan.semaphore().release()
    }
}
