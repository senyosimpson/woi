# Diary

Writing on the development of this runtime

## 20 August 2021

### Async task design

An issue I'm facing now is how to design the tasks and runtime. It requires an interesting design to
work, although I haven't entirely figured out the details. A 🧠 dump  of thoughts so far.

The leading issue is with tasks and running them in my simple runtime. A task currently is defined as

```rust
// Simplified from actual implementation
struct Task {
    future: Future<Output = ()>,
    executor: channel::Sender<Task>
}
```

The problem is the `future` field. The future's output is `()`. Instead what we want is something generic
like

```rust
struct Task<T> {
    future: Future<Output = T>.
    ...
}
```

Operating over a generic `T` means we can return anything from our future which is exactly what we want.
However, the runtime poses a problem. It's currently defined as

```rust
// Simplified from actual implementation
struct Runtime {
    scheduled_tasks: channel::Receiver<Task>,
    sender: channel::Sender<Task>
}
```

The problem here is that `Task` is needed in the fields. If we have a generic `T` then we would
need to have a `T` in the Runtime, i.e

```rust
struct Runtime<T> {
    scheduled_tasks: channel::Receiver<Task<T>>,
    sender: channel::Sender<Task<T>>
}
```

This obviously makes no sense. Your runtime is then constrained to only outputting one type of task.
Disaster.

The remedy here is to introduce some abstractions that help us deal with the issue. Exactly how that
is done is beyond my understanding as of right now but we can make some head way thinking about the
issue and taking inspiration from Tokio and async-std.

So addressing the `T` in the runtime: we need some abstraction that is parameterless. In async-std,
it is a `Runnable` and in Tokio it's a `ScheduledIo` (I think). How they are defined is for me to go
research. However, this allows you to write something like

```rust
struct Runtime {
    scheduled_tasks: channel::Receiver<Runnable>,
    ...
}
```

Now we've gotten rid of our generic paramater and `Runnable` is related to a task in some fashion.
In fact, a `Runnable` is a handle to a task. That makes sense. I'm not sure how `ScheduledIo` is
related to it's underlying task though.

Dealing with getting a `T` out of the runtime is still necessary. We can do this when we spawn a task.
So instead of having what we currently have

```rust
// runtime.rs
pub fn spawn<F>(&self, future: F)
where
    F: Future<Output = ()>
```

we would have

```rust
pub fn spawn<F, T>(&self, future: F) -> Task<T> {}
// or
pub fn spawn<F>(&self, future: F) -> Task<F::Output> {}
```

Here we're able to get a task that is awaitable and returns the output of the future. Again, how this
actually works I'm not sure.

## 23 August 2021

### Async task design

Both async-std and Tokio return `JoinHandle`'s. These are handles to the underlying task and returned
when a call to spawn happens. The idea is to have a `Runnable` and a `Task<T>`. Still not sure on all
the details. The idea would be something like

```rust
struct Runtime {
    scheduled_tasks: channel::Receiver<Runnable>,
    ...
}

impl Runtime {
    pub fn spawn<F>(&self, future: F) -> Task<F::Output> {
        // Create a runnable and a task
        let task = Task {

        };
        task
    }
}
```

There then has to be a link between a Runnable and a Task. I'm not sure how that would be facilitated
just yet. It seems that they're just using pointers to the same location in memory to do this but
I'm sure I can design something simple first.

## 24 August 2021

### Async task design

`async-task` has a nice idea for its `spawn` function where it takes in a scheduling function which
then passes that onto the runnable for scheduling.

Additionally, for the task and runnable, I can probably use a `Shared` construct. This is some data
that is shared with the `Runnable` and the `Task`. If you look at `async-task` they have roughly

```rust
struct Runnable {
    // Pointer to heap-allocated task
    ptr: NonNull
}

struct Task {
    // Raw task pointer
    ptr: NonNull,
    ...
}
```

The `spawn` function does something like

```rust
let ptr = NonNull::new(...);

let runnable = Runnable { ptr };
let task = Task { ptr, ... };
```

So we can see in this instance the `ptr` is effectively how they both reference the same piece of data.
If I can copy this without going the `ptr` route, that would be good. Just so I can get the roughest
thing working.

## 30 August 2021

### Async task design

I'm pretty confused as to how `task` and `runnable` are related to each other from a usage perspective.
For exmple, in `async-task` they have an example

```rust

let (runnable, task) = async_task::spawn(fut, schedule);
runnable.run()
smol::future::block_on(task)
```

I don't understand how you can run the `runnable` but then block on the `task`. It seems like both those
methods will poll the future, therefore effectively doing the same work. I need to look into this.

An interesting observation is that another example is as follows

```rust
let (runnable, task) = async_task::spawn(fut, schedule)
runnable.schedule() // instead of runnable.run()
smol::future::block_on(task)
```

I'm not sure what the difference here actually means from an implementation perspective.

***

I don't think its possible to build this out without using pointers. I'm still trying to think of a
design but I haven't been able to figure out a way to get out a generic `T`.
The problem is that the `Schedulable` needs to have some way of referring to the future. The future
operates over a generic `T` which then causes all the headaches we were trying to solve for as then
the `Schedulable` would also have a generic `T`.

Back to the drawing board!

## 7 November 2021

### Async task design

We're back after a long break! I've been reading source code like you can't believe. We've made progress,
we're doing good.

The original idea to use a shared construct between a `Schedulable` and a `Task` turned out to be infeasible.
No matter how you spin it, you can't get rid of the type `T` from propagating all over your code. To
get around this, you can make use of pointers that essentially allow for type erasure. I tried quite
a few different designs but could never get them to work (lifetimes ending my last attempt!). The end
design is very similar to the actual `async-task`. I think this is fine for now.

Moving forward, I need to start understanding how to keep state of the task is necessary.

## 22 November 2021

### Runtime design

A single-threaded executor requires two threads for the entire program. The single-threaded executor
just means the executor itself is single-threaded but the application will have to use two threads
in order to not block the main program.

## 4 January 2022

### Who knows

Another year, who would've guessed I'd still be working on this. Nonetheless, we've made a bunch of
progress.

### Progress update

So as it stands, we can run basic futures. There are a number of areas that need to be worked on however
but that will mostly be left to do after implementing networking I/O. Nonetheless, it is worth running
through how the executor is put together (mainly because I still find it kinda confusing).

We have a program

```rust
use woi;

fn main() {
    let rt = woi::Runtime::new();
    rt.block_on(async {
        let handle = rt.spawn(async {
            println!("Hello Senyo");
            5
        });

        let value = handle.await;
        println!("Value: {}", value);
    });
```

`block_on` takes in a future and blocks the thread on it, running it to completion. Similar to Tokio,
this is the runtime's entry point. All this does is loop through all the tasks spawned onto the executor
and polls them. When all the tasks are completed, the result is returned.

```rust
pub fn block_on<F: Future>(&self, future: F) -> F::Output {
    loop {
        match future.as_mut().poll(cx) {
            Poll::Ready(output) => return output,
            Poll::Pending => {
                // Go through all elements in the queue
                // When all have been processed, poll's the outer future again
                if let Some(task) = self.queue.borrow_mut().pop_front() {
                    task.poll();
                }
            }
        }
    }
}
```

Pretty neat. Now for a bit of a deep dive. `block_on` takes a future and we continously poll it until
it is ready. We refer to the future passed in as the *outermost future*. Why? Futures are often (but
not always) comprised of other futures. Polling the outermost future polls the inner futures. If any
of the inner futures are pending then we know the outermost future is pending.

In our runtime, inner futures are spawned onto the executor to be processed. We call them tasks.
This is done through a call to `rt.spawn()`. This pushes a task onto the runtime's queue. The user gets
a `JoinHandle` which is a handle to the inner future.

> A `task` *is* a future. It just holds some additional state used in the runtime.

From our example program, the outermost future spawns one inner future onto the runtime. The `block_on`
call will check if the outermost future is complete. On the initiai call to `poll` this will return
`Poll::Pending`. Why? Well, we haven't run the inner future yet - we only begin to process them if the
outermost future is in a pending state. Once all the inner futures are processed, we once again check
if the outermost future is complete. To reiterate, the outermost future is complete when *all* of the
inner futures are complete. If that is the case, we are done.

So now we can run futures and have an idea of how the runtime works but we still don't have any insight
into what is happening behind the scenes. Let's look at tasks. A `task` is a future with some additional
state.

```rust
struct Task {
    future: Future,
    state: State
}
```

When you call `rt.spawn()` you get a `JoinHandle` - a handle to that specific task on the executor.
When a handle is polled through an `.await` call, it checks whether the task it references is complete.

```rust
// This is pseudocode for the sake of explanation. Look through task/join.rs for the
// true implementation
impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let status = self.task.status;
        match status {
            Status::Done => {
                let output = self.task.get_output();
                return Poll::Ready(output);
            }
            _ => return Poll::Pending,
        }
    }
}
```

The underlying task is processed by the executor as shown earlier. So to walk through the entire process:

1. The outermost future is polled
2. This polls any inner futures
3. That occurs when there is a call to `.await`
4. That will call `poll()` on the `JoinHandle`
5. If the underlying task is incomplete, it will return `Poll::Pending`
6. The executor will then proceeed to process all the tasks
7. Once that is complete, back to 1

### Runtime design

The idea of using two threads stated above came from some code I read elsewhere. In that specific instance,
it made sense to use two threads because it's easy and simplified the code. However, it is isn't necessary
to use two threads. Given that using threads requires synchronization primitives, it's best to leave
it out if you don't need it. Hence, that is our plan.

### Wakers and reference counting

In most executor implementations, wakers are reference counted data structures. From my understanding,
this is the case so that the runtime can determine the logic behind the reference counting. For example,
in Tokio, a task starts with a reference count of 3. There are [alternative designs](https://github.com/rust-lang/rfcs/blob/master/text/2592-futures.md#rationale-drawbacks-and-alternatives-to-the-wakeup-design-waker)
to using reference counting but I think it's the best solution for multithreaded executors. Ours is a
single-threaded executor but for the sake of learning, I am still using a reference counted waker design.

### Task state

In both `async-task` and `tokio`, task state is stored as an `AtomicUsize`. This allows access to be
synchronized. Given this, they've encoded the state using bitmasks. In my design, I don't need this
and could implement the same thing with an enum. I was planning on opting for this design but I have
little experience with bitmasks (and will need it when working with epoll) so once again, I might as
well learn.

### Task state update

Turns out it's pretty straightforward. All we do is have certain bits represent a value. If we want
to check if a bit is set, we AND the bit we care about with the state. This will evaluate to 0 for
all bits *not* of interest. For the bit of interest, if it is set, we get back 1 and if not 0.
If we want to check or set bits, we OR the respective bitmasks. Since all the irrelevant bits will
be set to 0, it won't change what is currently set.

## 7 January 2022

### Pinning futures

Futures can be pinned on the heap or the stack. Pinning them on the stack is considered unsafe it does
not gaurantee that the future will be held at a stable memory location. In Tokio and async-std, they
pin futures to the stack but I had no idea why. Researching, I came across a good explanation in this
[Reddit thread](https://www.reddit.com/r/rust/comments/pd4ygo/when_to_pin_on_stack_when_to_pin_on_heap/haovu6q/).
In summary, since the future lives on the heap, anything pinned on the stack actually gets pinned on
the heap (since the future owns all the data). This gave me the clarity that I could indeed pin futures
on the stack as we allocate memory for the future on the heap at creation.

Note: I will admit, my mental model here is still not the strongest

### Task wakers

When writing the `block_on` function, I realised the waker used is different from that used in a task.
This warped my mental quite significantly.

1. Why are they different?
2. How and where do we even use the waker from the `block_on` function?

Let's start with question 2. My initial confusion came in with understand how the waker from the `block_on`
call propagates through futures. Turns out, my hunch was correct (as explained in another
[Reddit thread](https://www.reddit.com/r/rust/comments/icwhwb/does_async_await_create_its_own_wakers/)),
and that the context object is propagated from the parent future through all it's child futures. This
is done through the `poll` function as expected. This means that the waker we create in `block_on` will
propagate through to the relevant future - the `JoinHandle`.

The `JoinHandle` is responsible for polling whether the underlying task is complete.

```rust
// This is pseudocode for the sake of explanation. Look through task/join.rs for the
// true implementation
impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let status = self.task.status;
        match status {
            Status::Done => {
                let output = self.task.get_output();
                return Poll::Ready(output);
            }
            _ => return Poll::Pending,
        }
    }
}
```

The `JoinHandle` future needs to be woken up when the underlying task is complete, meaning that the underlying
task needs to be able to wake it up. How is that achieved? By storing the `JoinHandle` waker in the task
and invoking `wake/wake_by_ref` when it is complete. Our new implementation of the Future trait

```rust
// This is pseudocode for the sake of explanation. Look through task/join.rs for the
// true implementation
impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let status = self.task.status;
        match status {
            Status::Done => {
                let output = self.task.get_output();
                return Poll::Ready(output);
            }
            _ => {
                // NOTE: Here we store the waker in the task
                self.task.register_waker(cx.waker())
                return Poll::Pending
            }
        }
    }
}
```

Next question is why are they different? Here I'm still improving my mental mode. The gist of it is
that the runtime's threads get put to sleep if there is no work to do. However, they need to be woken
up in the event new work is queued. The waker in the `block_on` `wake` function performs this wake up
when invoked. The woken up thread will then fetch the output of the task.

## 24 January 2022

I've written way too much code. Hopefully I remember everything I need to document.

### Reactor design

At present, the reactor design is simple (as I don't have many requirements). It was defined as

```rust
// This is simplified for the sake of example
struct Reactor {
    poll: Epoll,
    // Storage for all the IO resources
    sources: Slab<IoSource>,
    // Stores the events on a call to `poll`
    events: Events
}
```

The part that took me some time to understand was the difference between `sources` and `events`. The
`events` field is a vector to store the results of a call to `epoll_wait`. However, we still need to
keep an active list of what resources we are managing. That is the point of `sources`. An event in
`events` is linked to a resource in `sources` through a `Token`. When we register an IO resource with
epoll, we give the `event` and the `source` the token

```rust
struct Token(usize);

pub fn register(&mut self, io: RawFd, interest: Interest) -> io::Result<IoSource> {
    let entry = self.sources.vacant_entry();
    // entry.key() returns the index in the slab where this entry points to
    // we store that key in the Token
    let token = Token(entry.key());
    let io_source = IoSource {
        // Omitted irrelevant fields
        token
    };

    self.poll.add(io, interest, token.clone())?;

    Ok(io_source)
}
```

As we can see, we register interest in the IO resource with epoll and then save the corresponding
source in the reactor. Its utility is discussed in the next section.

### IO type design

Being naive, I thought this would be a fairly straightforward process. As it turns out with everything
computing, it took much longer and was definitely more complicated than I had expected. I'll go through
implementing a `TcpStream`.

First off, why do we need our own `TcpStream`? As you probably suspect, it needs to integrate with our
system (i.e epoll). So we need something like

```rust
struct TcpStream {
    inner: std::net::TcpStream
}
```

In order for this to work, we need a bridge the connects the `TcpStream` to our reactor. In `async-std`
the bridge is named `Async` and in `Tokio` it's `PollEvented`. For lack of a better name, I've called
it `Pollable`. The above code is now

```rust
struct TcpStream {
    inner: Pollable<std::net::TcpStream>
}
```

where a `Pollable`

```rust
struct Pollable<T> {
    io: T
}
```

The `Pollable` is responsible for registering the underlying IO resource in the reactor. In reality,
a `Pollable` object is defined as

```rust
struct Pollable<T> {
    io: T,
    source: IoSource
}
```

The famed `IoSource` is back! Our `Pollable` object contains both the actual IO object (i.e `std::net::TcpStream`)
and an `IoSource` (which is reference counted). The `IoSource` itself has a number of methods (many to
related reading and writing of data) since it contains the data for making those judgements.

### Runtime context

We want to:

1. Spawn all tasks onto the *same* executor
2. Be able to access the executor from anywhere in the program

The runtime has a handle which is used to interact with it. Through the handle, tasks are spawned onto
the executor. It is defined as

```rust
struct Handle {
    spawner: Spawner,
    // This the handle of the reactor instance
    io: IoHandle
}
```

To meet our requirements, we store the handle in thread local storage and fetch the handle from there
whenever we need to interact with the runtime (and the reactor).

### Cloning Epoll

I ran into a fun bug with my epoll library. The `Epoll` object is defined as

```rust
#[derive(Clone)]
Epoll {
    fd: RawFd
}
```

Since `RawFd` is an integer, a clone of this will just copy the integer value. This is kind of what
we want: a copy of `Epoll` points to the same underlying epoll instance. We've also implemented the
`Drop` trait for `Epoll`

```rust
impl Drop for Epoll {
    fn drop(&self) {
        // This performs a syscall, closing the file descriptor
        close(self.fd)
    }
}
```

Herein lies the problem. Since we can clone the `Epoll` object, when *any* of the clones go out of scope,
it sends a syscall to close the epoll instance. The easy fix is to wrap `Epoll` in a reference counted
structure when it is used. This is what I've done. However, maybe you want don't want to do that. The
solution here is the `dup()`/`dup2()` syscalls, usually performed through `fcntl`.

## 31 January 2022

### Channels

After playing around with Elixir, I've had a growing interest in messaging systems. I'm really not sure
why, I just find them interesting. Given that, I've decided to expand the scope here slightly and implement
channels so that tasks can communicate with each other. This is probably the only other feature I'll
implement (which leaves *plenty* (i.e almost everything) to be desired).

I took to reading through [Flume](https://github.com/zesterer/flume), [async-channel](https://github.com/smol-rs/async-channel)
and [Tokio](https://github.com/tokio-rs/tokio/tree/master/tokio/src/sync/mpsc) to figure out how to
put it together. While I have some gaps on how to implement, I thought I'd just walk through the design
options and which one I'll be going with.

Both Flume and async-channel have a central object (`Shared` for the former, `Channel` for the latter)
shared between the `Sender` and `Receiver`. All methods for sending and receiving messages are attached
to the central object.

```rust
// All of the code here is psuedocode

// ===== Channel =====

struct Channel<T> {}

impl<T> Channel<T> {
    fn send(&self, message: T) -> Result<(), SendError> {}
    fn recv(&self) -> Result<T, RecvError> {}
}

// ===== Sender =====

struct Sender<T> {
    inner: Channel<T> 
}

impl<T> Sender<T> {
    fn send(&self, message: T) -> Result<(), SendError> {
        self.inner.send(message)
    }
}

// ===== Receiver =====

struct Receiver<T> {
    inner: Channel<T>
}

impl<T> Receiver<T> {
    fn recv(&self) -> Result<T, RecvError> {
        self.inner.recv()
    }
}
```

This design neat and straightforward. However, one thing I *personally* don't like about it
is that the shared structure has both send and receive methods tied to it. In my mental model, I view
the send and receive sides of a channel as two entirely separate entities. I'd like for my API to reflect
that. Tokio does this nicely.

```rust
// All of the code here is psuedocode

// ===== Channel =====
struct Channel<T> {}

impl<T> Channel<T> {
    // ...
    // ...
}

// ===== Sender =====

struct Sender<T> {
    chan: Tx<T>
}

struct Tx<T> {
    inner: Channel<T>
}

impl<T> Sender<T> {
    fn send(&self, message: T) -> Result<(), SendError> {
        self.chan.send(message)
    }
}

impl<T> Tx<T> {
    fn send(&self, message: T) -> Result<(), SendError> {}
}

// ===== Receiver =====

struct Receiver<T> {
    chan: Rx<T>
}

struct Rx<T> {
    inner: Channel<T>
}

impl<T> Receiver<T> {
    fn recv(&self) -> Result<T, RecvError> {
        self.chan.recv()
    }
}

impl<T> Rx<T> {
    fn recv(&self) -> Result<T, RecvError> {}
}
```

The downside of this design is that it has more layers, making it more complicated to understand. Additionally,
the purpose of `Channel` becomes debatable since there's not much implemented on its level.

## 02 February 2022

### Channels

As it turns out, the purpose of `Channel` was basically reduced to nothing. Adding the extra layers
didn't improve the design in any meaningful way. The constraints are entirely different from Tokio
which has multiple types of channels, meaning having the additional layer helps design the higher
level abstractions. I don't have this problem.

Given this, I will go with design one.

## 11 February 2022

### Timers

Whilst developing channels, I was thinking of potential failure modes. For one of them, I needed to write
some code which put one half of the channel asleep. That lead me down a rabbit hole to implement a `sleep`
function. Turns out, almost all of the infrastructure we needed was already there making it pretty
straightforward to implement.

I used the `timerfd` syscalls and registered the resulting file descriptor in epoll to implement it.
Let's walk through the design.

We have a function `sleep` that is used as such

```rust
use woi::time::sleep;
use std::time::Duration;

sleep(Duration::from_secs(2)).await;
```

It is defined as

```rust
async fn sleep() -> Sleep {}
```

where `Sleep` is a future.

```rust
struct Sleep {
    inner: Pollable<Timer>
}
```

The `Timer` is responsible for creating a new `Timer` instance. This creates the timer (i.e performs
the syscall `timer_create`) and sets its deadline (i.e the time at which it will fire an event).

The only complexity came in with reasoning about how to implement the future, even though the implementation
is straightforward (like 3 lines, lol). In the case of a timer, when the event fires, it becomes readable.
However, we don't actually care about reading the resource when it is complete, we just care about knowing
it is readable. Luckily, we have a function just for that, `poll_readable` which is implemented on a
`Pollable`. Thus given everything we've done so far, implementing the future becomes boring.

```rust
impl Future for Sleep {
    Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match ready!(self.inner.poll_readable(cx)) {
            Ok(()) => Poll::Ready(()),
            Err(e) => panic!("timer error: {}", e)
        }
    }
}
```
