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
```
