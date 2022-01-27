mod addr;

mod tcp;
pub use tcp::TcpStream;

// Re-exports
pub use std::net::{
    IpAddr,
    Ipv4Addr,
    Ipv6Addr,
    Shutdown,
    SocketAddr,
    SocketAddrV4,
    SocketAddrV6,
    AddrParseError
};
