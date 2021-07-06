use std::{future::Future, io, net::{SocketAddr, SocketAddrV4, SocketAddrV6}};

use crate::future;

type ReadyFuture<T> = future::Ready<io::Result<T>>;

pub trait ToSocketAddrs {
    type Iter: Iterator<Item = SocketAddr>;
    type Future: Future<Output = io::Result<Self::Iter>>;

    fn to_socket_addrs(&self) -> Self::Future;
}

impl ToSocketAddrs for SocketAddr {
    type Iter = std::option::IntoIter<SocketAddr>;
    type Future = ReadyFuture<Self::Iter>;

    fn to_socket_addrs(&self) -> Self::Future {
        let iter = Some(*self).into_iter();
        future::ok(iter)
    }
}

impl ToSocketAddrs for SocketAddrV4 {
    type Iter = std::option::IntoIter<SocketAddr>;
    type Future = ReadyFuture<Self::Iter>;

    fn to_socket_addrs(&self) -> Self::Future {
        let addr = SocketAddr::V4(*self);
        ToSocketAddrs::to_socket_addrs(&addr)
    }
}

impl ToSocketAddrs for SocketAddrV6 {
    type Iter = std::option::IntoIter<SocketAddr>;
    type Future = ReadyFuture<Self::Iter>;

    fn to_socket_addrs(&self) -> Self::Future {
        let addr = SocketAddr::V6(*self);
        ToSocketAddrs::to_socket_addrs(&addr)
    }
}