# Notes

Waking up a thread (notify) on a blocked poll/epoll

* [Example logic](https://github.com/smol-rs/polling/blob/952fccb2f56fbc6eaf7d6d80de5e6704a76f4c21/src/epoll.rs#L185)
* [Stack Overflow](https://stackoverflow.com/questions/12050072/how-to-wake-up-a-thread-being-blocked-by-select-poll-poll-function-from-anothe)

## Wakers

* https://boats.gitlab.io/blog/post/wakers-i/
* https://github.com/aturon/rfcs/blob/e7eaea194994da28bde2c36d78fedf50e79b4bcf/text/2592-futures.md#waking-up
* https://github.com/aturon/rfcs/blob/e7eaea194994da28bde2c36d78fedf50e79b4bcf/text/2592-futures.md#rationale-drawbacks-and-alternatives-to-the-wakeup-design-waker
