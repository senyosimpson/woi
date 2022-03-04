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

## 4 March 2022

It's been some time since I last documented changes. Here we go.

### Diagnostics

I started using [`tracing`](https://github.com/tokio-rs/tracing) to capture diagnostics about my runtime.
This has proven to be invaluable in debugging some issues (namely reference counting and dropping resources).
The glory of running a single-threaded runtime is that you can write code that actually runs in a deterministic
fashion. For example

```rust
let rt = Runtime::new();
rt.block_on(async {
    let (tx, rx) = mpsc::channel();
    woi::spawn(async {
        let tx = tx.clone();
        println!("Sending message from task 1");
        tx.send("task 1: fly.io").unwrap()
    });

    woi::spawn(async move {
        println!("Sending message from task 2 after sleeping");
        sleep(Duration::from_secs(1)).await;
        println!("Done sleeping. Sending message from task 2");
        tx.send("task 2: hello world").unwrap();
        println!("Sent message!");
    });

    let h2 = woi::spawn(async move {
        println!("Received message: {}", rx.recv().await.unwrap());
        println!("Received message: {}", rx.recv().await.unwrap());
    });

    h2.await.unwrap();
});
```

We know that tasks are spawned in order and are processed in order. Therefore, the order of output
here will be

```
Sending message from task 1
Sending message from task 2 after sleeping
Received message: task 1: fly.io
Done sleeping. Sending message from task 2
Received message: task 2: hello world
```

In steps

1. The first task runs and sends the message. It does not have to yield at any point.
2. The second task runs. It prints that it'll send a message after sleeping. It then yields.
3. The third task runs. It receives the first message that was sent from task 1. It then yields as
   it is attempting to receive a message but none has been sent yet.
4. The second task wakes up after sleeping and sends its message
5. The third task wakes up after receiving a message and prints it out

We can then quite easily, track the lifecycle of a task. I have some ways to go to improve this but
this is a raw output of the current diagnostics

<details>
<summary>Diagnostic output</summary>

<!-- markdownlint-disable MD013-->
```
2022-03-04T08:31:58.110903Z DEBUG woi::runtime::runtime: Polling `block_on` future
2022-03-04T08:31:58.110939Z DEBUG woi::runtime::runtime: Task 1: Spawned
2022-03-04T08:31:58.110951Z DEBUG woi::task::join: Task 1: Dropping JoinHandle
2022-03-04T08:31:58.110962Z DEBUG woi::task::state: Task 1: Decr ref count. Value: 1
2022-03-04T08:31:58.110973Z DEBUG woi::runtime::runtime: Task 2: Spawned
2022-03-04T08:31:58.110981Z DEBUG woi::task::join: Task 2: Dropping JoinHandle
2022-03-04T08:31:58.110988Z DEBUG woi::task::state: Task 2: Decr ref count. Value: 1
2022-03-04T08:31:58.110996Z DEBUG woi::runtime::runtime: Task 3: Spawned
2022-03-04T08:31:58.111007Z DEBUG woi::task::join: Task 3: JoinHandle is complete: false
2022-03-04T08:31:58.111017Z DEBUG woi::runtime::runtime: Task 1: Popped off executor queue and running
2022-03-04T08:31:58.111028Z DEBUG woi::task::state: Task 1: Transitioned to running. State: State { scheduled=false, running=true, complete=false, has_join_handle=false, has_join_waker=false, ref_count=1 }
Sending message from handle 1
2022-03-04T08:31:58.111043Z DEBUG woi::channel::mpsc: Dropping sender
2022-03-04T08:31:58.111053Z DEBUG woi::task::state: Task 1: Transitioned to complete. State: State { scheduled=false, running=false, complete=true, has_join_handle=false, has_join_waker=false, ref_count=1 }
2022-03-04T08:31:58.111063Z DEBUG woi::task::state: Task 1: Decr ref count. Value: 0
2022-03-04T08:31:58.111073Z DEBUG woi::task::raw: Task 1: Deallocating
2022-03-04T08:31:58.111082Z DEBUG woi::runtime::runtime: Task 2: Popped off executor queue and running
2022-03-04T08:31:58.111090Z DEBUG woi::task::state: Task 2: Transitioned to running. State: State { scheduled=false, running=true, complete=false, has_join_handle=false, has_join_waker=false, ref_count=1 }
Sending message from handle 2 after sleeping
2022-03-04T08:31:58.111106Z DEBUG woi::io::reactor: Registering task in epoll
2022-03-04T08:31:58.111122Z DEBUG woi::io::io_source: Invoking poll_readable
2022-03-04T08:31:58.111133Z DEBUG woi::task::state: Task 2: Incr ref count. Value: 2
2022-03-04T08:31:58.111144Z DEBUG woi::io::io_source: poll_readable returned Poll::Pending
2022-03-04T08:31:58.111153Z DEBUG woi::task::raw: Task pending
2022-03-04T08:31:58.111162Z DEBUG woi::task::state: Task 2: Transitioned to idle. State: State { scheduled=false, running=false, complete=false, has_join_handle=false, has_join_waker=false, ref_count=2 }
2022-03-04T08:31:58.111171Z DEBUG woi::task::state: Task 2: Decr ref count. Value: 1
2022-03-04T08:31:58.111179Z DEBUG woi::runtime::runtime: Task 3: Popped off executor queue and running
2022-03-04T08:31:58.111187Z DEBUG woi::task::state: Task 3: Transitioned to running. State: State { scheduled=false, running=true, complete=false, has_join_handle=true, has_join_waker=true, ref_count=2 }
Received message: handle 1: fly.io
2022-03-04T08:31:58.111199Z DEBUG woi::task::state: Task 3: Incr ref count. Value: 3
2022-03-04T08:31:58.111207Z DEBUG woi::task::raw: Task pending
2022-03-04T08:31:58.111215Z DEBUG woi::task::state: Task 3: Transitioned to idle. State: State { scheduled=false, running=false, complete=false, has_join_handle=true, has_join_waker=true, ref_count=3 }
2022-03-04T08:31:58.111224Z DEBUG woi::task::state: Task 3: Decr ref count. Value: 2
2022-03-04T08:31:58.111232Z DEBUG woi::runtime::runtime: Polling `block_on` future
2022-03-04T08:31:58.111239Z DEBUG woi::task::join: Task 3: JoinHandle is complete: false
2022-03-04T08:31:58.111251Z DEBUG woi::runtime::runtime: Parking on epoll
2022-03-04T08:31:59.111167Z DEBUG woi::io::epoll: Epoll: Received 1 events
2022-03-04T08:31:59.111216Z DEBUG woi::io::reactor: Epoll: processing Event { token=0, interest=Interest { epollin=true, epollout=false, epollpri=false, epollhup=false, epollrdhup=false } }
2022-03-04T08:31:59.111251Z DEBUG woi::task::raw: Task 2: Waking raw task
2022-03-04T08:31:59.111269Z DEBUG woi::task::state: Task 2: Transitioned to scheduled. State: State { scheduled=true, running=false, complete=false, has_join_handle=false, has_join_waker=false, ref_count=1 }
2022-03-04T08:31:59.111288Z DEBUG woi::task::state: Task 2: Incr ref count. Value: 2
2022-03-04T08:31:59.111306Z DEBUG woi::task::state: Task 2: Decr ref count. Value: 1
2022-03-04T08:31:59.111326Z DEBUG woi::runtime::runtime: Task 2: Popped off executor queue and running
2022-03-04T08:31:59.111342Z DEBUG woi::task::state: Task 2: Transitioned to running. State: State { scheduled=false, running=true, complete=false, has_join_handle=false, has_join_waker=false, ref_count=1 }
2022-03-04T08:31:59.111361Z DEBUG woi::io::io_source: Invoking poll_readable
2022-03-04T08:31:59.111380Z DEBUG woi::io::io_source: poll_readable returned Poll::Ready(ok)
2022-03-04T08:31:59.111398Z DEBUG woi::io::reactor: Deregistering task from epoll
Done sleeping. Sending message from handle 2
2022-03-04T08:31:59.111425Z DEBUG woi::task::raw: Task 3: Waking raw task by ref
2022-03-04T08:31:59.111440Z DEBUG woi::task::state: Task 3: Transitioned to scheduled. State: State { scheduled=true, running=false, complete=false, has_join_handle=true, has_join_waker=true, ref_count=2 }
2022-03-04T08:31:59.111458Z DEBUG woi::task::state: Task 3: Incr ref count. Value: 3
2022-03-04T08:31:59.111473Z DEBUG woi::channel::mpsc: Dropping sender
2022-03-04T08:31:59.111490Z DEBUG woi::task::state: Task 2: Transitioned to complete. State: State { scheduled=false, running=false, complete=true, has_join_handle=false, has_join_waker=false, ref_count=1 }
2022-03-04T08:31:59.111508Z DEBUG woi::task::state: Task 2: Decr ref count. Value: 0
2022-03-04T08:31:59.111524Z DEBUG woi::task::raw: Task 2: Deallocating
2022-03-04T08:31:59.111539Z DEBUG woi::runtime::runtime: Task 3: Popped off executor queue and running
2022-03-04T08:31:59.111554Z DEBUG woi::task::state: Task 3: Transitioned to running. State: State { scheduled=false, running=true, complete=false, has_join_handle=true, has_join_waker=true, ref_count=3 }
Received message: handle 2: hello world
2022-03-04T08:31:59.111579Z DEBUG woi::channel::mpsc: Dropping receiver
2022-03-04T08:31:59.111596Z DEBUG woi::task::state: Task 3: Decr ref count. Value: 2
2022-03-04T08:31:59.111612Z DEBUG woi::task::state: Task 3: Transitioned to complete. State: State { scheduled=false, running=false, complete=true, has_join_handle=true, has_join_waker=true, ref_count=2 }
2022-03-04T08:31:59.111630Z DEBUG woi::task::state: Task 3: Decr ref count. Value: 1
2022-03-04T08:31:59.111645Z DEBUG woi::runtime::runtime: Polling `block_on` future
2022-03-04T08:31:59.111660Z DEBUG woi::task::join: Task 3: JoinHandle is complete: true
2022-03-04T08:31:59.111677Z DEBUG woi::task::join: Task 3: JoinHandle ready
2022-03-04T08:31:59.111693Z DEBUG woi::task::join: Task 3: Dropping JoinHandle
2022-03-04T08:31:59.111708Z DEBUG woi::task::state: Task 3: Decr ref count. Value: 0
2022-03-04T08:31:59.111723Z DEBUG woi::task::raw: Task 3: Deallocating
2022-03-04T08:31:59.111741Z DEBUG woi::runtime::context: Dropping enter guard
Finished
2022-03-04T08:31:59.111764Z DEBUG woi::io::epoll: Drop: epoll_fd=3
```
<!-- markdownlint-restore -->
</details>

We can (somewhat) follow the lifecycle of a task through the program. I had to implement task ids as
well. Due to my own tendency to over engineer (and for learning purposes), it turned out to be pretty
non-trivial to do. The reason being, I wanted a static variable that is modifiable so that I could keep
the tracking of the IDs with the struct that is responsible for generating them, rather than at the runtime.
Usually, this is done with atomics. For [example](https://os.phil-opp.com/async-await/#executor-with-waker-support)

```rust
struct TaskId(u64);

impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}
```

However, since this is single-threaded, I didn't want to pay the cost of atomics. I figured, I could
replicate it without. The key to doing so is to realise is that `AtomicU64` uses interior mutability.
That is why we're able to update it even though it is a constant (since we're updating internal state,
not reassigning the `NEXT_ID` variable). My solution

```rust
/// A monotonic counter that is updated through interior
/// mutability. Allows it to be used as a static while still
/// being able to be updated
#[derive(Default)]
struct Counter(Cell<u64>);

#[derive(Clone, Copy)]
pub(crate) struct TaskId(u64);

// ===== impl Counter =====

// Implement sync for counter to enable it to be used as
// a static. It is safe to do so because we aren't sharing
// it across threads
unsafe impl Sync for Counter {}

impl Counter {
    const fn new() -> Counter {
        Counter(Cell::new(0))
    }

    pub fn incr(&self) -> u64 {
        let prev = self.0.get();
        let new = prev + 1;
        self.0.set(new);
        new
    }
}

// ===== impl TaskId =====

impl TaskId {
    pub fn new() -> Self {
        static ID: Counter = Counter::new();
        TaskId(ID.incr())
    }
}
```

Nice and simple. The `TaskId` is added to the header of the task and so we can easily fetch it whenever
we need.

The easier alternative would have been to keep the state of the ID in the runtime and pass it through
the `spawn` function. However, I think this may have made for an uglier API (I'm not sure).

### Reference counting

After implementing diagnostics, I was able to figure out how reference counts were being updated throughout
the lifetime of a task. This revealed two bugs (one which I knew).

1. The reference count was not decreased on a call to `wake`
2. The reference count was not increased on a call to `schedule`

Starting with the second case. I had implemented some code that for the first time, rescheduled a task
more than once. This ended up causing my task to get deallocated before completion. A task starts with
a reference count of 2, 1 belonging to the `JoinHandle` and another to the task on the executor. They
both point to the same point in memory where the tasks state is held. The task on the executor is created
when it is spawned onto the runtime and dropped after it yields/completes. The bug is shown through the
steps

1. Start with reference count of 2
2. Run the task in the queue (it's called a `Task`)
3. When the `Task` yields/completes, decrement the reference count to 1. It now equals 1
4. Reschedule the task
5. When the `Task` yields/completes, decrement the reference count by 1. It now equals 0
6. Deallocate task

When trying to retrieve the output, it would give gibberish since the task is deallocated. The solution
is to increment the reference count when a task is rescheduled since that creates a new `Task` which
is pushed onto the executor queue.

The first issue I knew was a problem since it is in the contract of `wake`. It takes ownership of the
waker, therefore it had to decrement the reference count. However, since I didn't increment the reference
count when scheduling a new task, decrementing the count on `wake` was causing my code to fail. With
the second issue fixed, I just had to add the reference decrement.

### Interior mutability runtime violation

One of the most interesting bugs I've come across in recent times. My assumption is that it was due to
what is called, lifetime extension. For an example and explanation of this, read this interesting [post](https://fasterthanli.me/articles/a-rust-match-made-in-hell).

In my case, I had the code

```rust
while let Some(task) = self.queue.borrow_mut().pop_front() {
    task.run();
}
```

It started breaking when I implemented channels. The `Sender` stores a waker that calls `wake` whenever
it sends a message. When `wake` is called, it pushes the task onto the *runtime*'s queue. That is the
*same* that is being borrowed above. Therefore, we

1. Borrow the queue to get the task and run it
2. The task also needs to borrow the queue when it sends a message

This breaks Rust's rules and crashes at runtime. The fix is simply

```rust
loop {
    let task = self.queue.borrow_mut().pop_front();
    match task {
        Some(task) => {
            task.run()
        }
        None => break,
    }
}
```

Weird right? In the first version, we have one expression. This means that during the entire `while let`
loop, the queue is being borrowed. That is why when we need to borrow it again during the `task.run()`,
we violate the rules. In the second version, we only borrow the queue to set `task`. That means
any borrow in `task.run()` will be valid.

### Dropping epoll and thread local context

At some point, I realised that some objects were not being dropped at the end of execution. I realised
because I had a `Drop` implementation for `Epoll` which just printed out it was getting dropped. After
doing some digging, I found that the issue was because the thread local context variable still held a
handle to the runtime which contained epoll. Thread local storage isn't subject to clean up at the end
of program execution (as far as I know). This led me down a bit of a rabbit hold and I ended up implementing
an RAII guard to do the clean up. I unashamedly stole this from Tokio.

We have a guard

```rust
pub(crate) struct EnterGuard;

impl Drop for EnterGuard {
    fn drop(&mut self) {
        tracing::debug!("Dropping enter guard");
        CONTEXT.with(|ctx| {
            ctx.borrow_mut().take();
        })
    }
}
```

It's only purpose is to take the value outside of `CONTEXT`, thereby dropping it when `Drop` completes
executing. We call it in the `block_on` function (the entry point of the runtime). When the `block_on`
function completes, the enter guard is dropped, clearing the thread local context.

```rust
pub fn block_on<F: Future>(&self, future: F) -> F::Output {
    // Enter runtime context
    let _enter = context::enter(self.handle.clone());
    self.inner.borrow_mut().block_on(future)
}
```

### No op waker

For a while now, I've wondered if a context and waker is necessary for the `block_on` call. This waker
is responsible for unparking a parked thread. It's used in multi-threaded runtimes so that one thread
can signal to another to wakeup. However, for a single-threaded runtime, unparking yourself is impossible.
I wasn't sure if it was necessary for any other functionality. One day I was looking at [Glommio](https://github.com/DataDog/glommio)
and figured, if there was a necessity for this waker, it would be in there. To my amusement, they also
passed in a dummy waker. I just formalised it here into its own struct and called it `NoopWaker`

```rust
struct NoopWaker;

impl NoopWaker {
    fn waker() -> RawWaker {
        fn no_op(_: *const ()) {}
        fn clone(_: *const ()) -> RawWaker {
            NoopWaker::waker()
        }

        let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
        RawWaker::new(0 as *const (), vtable)
    }
}
```

### Panics as errors

As I was reading `async-task`, I remembered them having panic guards but never understood the reasoning
behind it. I was looking into it and discovered its so that panics in tasks do not crash the entire
program. This makes sense, we want async tasks to behave similar to threads - if they crash, only that
task is impacted. The rest of the program can operate. We achieve this through panic guards and catching
panics.

#### Catching panics

Catching panics uses [`panic::catch_unwind`](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html).
This returns a result that contains an error if the code within it panicked. We can then propagate this
panic up to the `JoinHandle`. Panic guards are used to implement some behaviour while a panic is unwinding.
When a panic occurs, Rust unwinds in the order of declaration and calls `Drop` on all the resources.
We can take advantage of this as I will show later.

There are two places we need to take care of panics:

1. When we drop a future
2. When we poll a future

When we drop a future, it could panic for... reasons. We can catch this

```rust
let res = panic::catch_unwind(|| {
    self.drop_future_or_output()
});
```

Nice and simple. When we poll a future, we also need to take care of this. Someone may write code like

```rust
woi::spawn(async {
    panic!("BE GONE!")
})
```

To handle this, we have to use a panic guard. First, to show you how I've implemented it (also copied from
Tokio)

```rust
fn poll_inner(status: &mut Status<F>, cx: &mut Context) -> Poll<()> {
    use std::panic;

    struct Guard<'a, F: Future> {
        status: &'a mut Status<F>,
    }

    impl<'a, F: Future> Drop for Guard<'a, F> {
        fn drop(&mut self) {
            // If polling the future panics, we want to drop the future/output
            // If dropping the future/output panics, we've wrapped the entire method in
            // a panic::catch_unwind so we can return a JoinError
            self.status.drop_future_or_output()
        }
    }

    let res = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let guard = Guard { status };
        let res = guard.status.poll(cx);
        // Successfully polled the future. Prevent the guard's destructor from running
        mem::forget(guard);
        res
    }));
    ...
    ...
}
```

What happens here is we wrap a call to `poll` in a panic guard. If the call to poll (`guard.status.poll`)
panics, it will start unwinding the variables within the `panic::catch_unwind`.  Since we've created
a guard, the guard's `Drop` implementation will run. This will drop the future. The error will be returned
to the `JoinHandle`.
