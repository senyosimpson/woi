# senio

[epoll]: https://man7.org/linux/man-pages/man7/epoll.7.html
[Mio]: https://github.com/tokio-rs/mio
[Tokio]: https://tokio.rs/
[async-std]: https://async.rs
[Polling]: https://github.com/smol-rs/polling

Senio is an event notification library that interfaces with [epoll]. It takes inspiration from
multiple sources:

1. [Mio] - A cross-platform event notification library used in [Tokio]
2. [Polling] - Another cross-platform event notification library used in [async-std]

Resources I looked into in making this:

1. [Epoll, Kqueue and IOCP Explained with Rust](https://cfsamsonbooks.gitbook.io/epoll-kqueue-iocp-explained/)

## Example

```rust
use senio::{Events, Interest, Poll, Token};
use std::io::{self, Read};
use std::net::TcpListener;

fn main() -> io::Result<()> {
    let socket = TcpListener::bind("localhost:1313")?;
    socket
        .set_nonblocking(true)
        .expect("Failed to set to non-blocking mode");

    let poll = Poll::new()?;
    let mut events = Events::with_capacity(1024);
    poll.add(&socket, Interest::READABLE, Token(0))?;

    loop {
        // Block until at least one event is ready
        println!("Waiting for events!");

        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                Token(0) => {
                    println!("Woohooo! Matching token");
                    let (mut stream, _) = socket.accept()?;
                    println!("Accepted socket successfully!");
                    let mut buffer = [0; 4096];
                    stream.read(&mut buffer)?;
                    println!("Read data: {}", String::from_utf8_lossy(&buffer));
                    // Re-register
                    poll.modify(&socket, Interest::READABLE, Token(0))?;
                }
                _ => {
                    println!("Nothing here, continuing")
                }
            }
        }
    }
}
```
