// Mostly taken from Tokio
use std::{future::Future, io, net::{SocketAddr, SocketAddrV4, SocketAddrV6}};

use crate::future::Ready;

pub(crate) trait ToSocketAddrs {
    type Iter: Iterator<Item = SocketAddr>;
    type Future: Future<Output = io::Result<Self::Iter>>;

    fn to_socket_addrs(&self) -> Self::Future;
}

impl ToSocketAddrs for SocketAddr {
    type Iter = std::option::IntoIter<SocketAddr>;
    type Future = Ready<io::Result<Self::Iter>>;

    fn to_socket_addrs(&self) -> Self::Future {
        let iter = Some(*self).into_iter();
        Ready(Some(Ok(iter)))
    }
}

impl ToSocketAddrs for SocketAddrV4 {
    type Iter = std::option::IntoIter<SocketAddr>;
    type Future = Ready<io::Result<Self::Iter>>;

    fn to_socket_addrs(&self) -> Self::Future {
        let addr = SocketAddr::V4(*self);
        self::ToSocketAddrs::to_socket_addrs(&addr)
    }
}

impl ToSocketAddrs for SocketAddrV6 {
    type Iter = std::option::IntoIter<SocketAddr>;
    type Future = Ready<io::Result<Self::Iter>>;

    fn to_socket_addrs(&self) -> Self::Future {
        let addr = SocketAddr::V6(*self);
        self::ToSocketAddrs::to_socket_addrs(&addr)
    }
}