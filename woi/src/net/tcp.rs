// TcpListener, TcpStream, Incoming
use std::{
    io::{self, Read},
    pin::Pin,
    task::{Context, Poll},
};

use super::addr::ToSocketAddrs;
use crate::io::AsyncRead;

pub struct TcpListener {
    inner: std::net::TcpListener,
}

pub struct TcpStream {
    inner: std::net::TcpStream,
}

impl TcpListener {
    pub fn new(inner: std::net::TcpListener) -> io::Result<TcpListener> {
        inner.set_nonblocking(true)?;
        Ok(TcpListener { inner })
    }

    pub async fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<TcpListener> {
        let mut err = None;

        for addr in addr.to_socket_addrs().await? {
            match std::net::TcpListener::bind(addr) {
                Ok(listener) => return TcpListener::new(listener),
                Err(e) => err = Some(e),
            }
        }

        Err(err.unwrap_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "could not connect to any of the addresses",
            )
        }))
    }

    // pub async fn accept<A: ToSocketAddrs>(addr: A) -> io::Result<(net::TcpStream, SocketAddr)> {
    // let listener = TcpListener::bind(addr).await?;
    // // match listener.inner.accept() {

    // // }
    // }

    pub fn local_addr() {}

    pub fn ttl() {}

    pub fn set_ttl() {}
}

impl TcpStream {}

impl AsyncRead for TcpStream {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        match self.inner.read(buf) {
            Ok(n) => return Poll::Ready(Ok(n)),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
            Err(e) => return Poll::Ready(Err(e)) 
        }
        
    }
}
