// TcpListener, TcpStream, Incoming
use std::io;

use crate::io::AsyncRead;
use super::addr::ToSocketAddrs;

pub struct TcpListener {
    inner: std::net::TcpListener,
}

pub struct TcpStream {
    inner: std::net::TcpStream
}

impl TcpListener {
    pub fn new(inner: std::net::TcpListener) -> io::Result<TcpListener> {
        // Set to nonblocking
        inner.set_nonblocking(true)?;
        Ok(TcpListener { inner })
    }

    pub async fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<TcpListener> {
        let mut err  = None;

        for addr in addr.to_socket_addrs().await? {
            match std::net::TcpListener::bind(addr) {
                Ok(listener) => return TcpListener::new(listener),
                Err(e) => err = Some(e)
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
    
}