// TcpListener, TcpStream, Incoming
use std::{io, net::SocketAddr};

use crate::addr::ToSocketAddrs;

pub struct TcpListener {
    inner: std::net::TcpListener,
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

        err
    }

    pub async fn accept<A: ToSocketAddrs>(addr: A) -> io::Result<(TcpStream, SocketAddr)> {
        let listener = TcpListener::bind(addr).await?;
        match listener.accept() {

        }
    }

    pub fn local_addr() {}

    pub fn ttl() {}

    pub fn set_ttl() {}
}
