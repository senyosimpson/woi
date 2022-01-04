# Diary

Writing on the development of this runtime

## Tasks and the runtime - 20/08/2021

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

### 23/08/2021

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

### 24/08/2021

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

### 30/08/2021

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

### 07/11/2021

We're back after a long break! I've been reading source code like you can't believe. We've made progress,
we're doing good.

The original idea to use a shared construct between a `Schedulable` and a `Task` turned out to be infeasible.
No matter how you spin it, you can't get rid of the type `T` from propagating all over your code. To
get around this, you can make use of pointers that essentially allow for type erasure. I tried quite
a few different designs but could never get them to work (lifetimes ending my last attempt!). The end
design is very similar to the actual `async-task`. I think this is fine for now.

Moving forward, I need to start understanding how to keep state of the task is necessary.

### 22/11/2021

A single-threaded executor requires two threads for the entire program. The single-threaded executor
just means the executor itself is single-threaded but the application will have to use two threads
in order to not block the main program.

#### Update: 04/01/2022

The idea of using two threads stated above came from some code I read elsewhere. In that specific instance,
it made sense to use two threads because it's easy and simplified the code. However, it is isn't necessary
to use two threads. Given that using threads requires synchronization primitives, it's best to leave
it out if you don't need it. Hence, that is our plan.

### 04/01/2022

Another year, who would've guessed I'd still be working on this. Nonetheless, we've made a bunch of
progress.

#### 1000 foot view

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

#### Wakers and reference counting

In most executor implementations, wakers are reference counted data structures. From my understanding,
this is the case so that the runtime can determine the logic behind the reference counting. For example,
in Tokio, a task starts with a reference count of 3. There are [alternative designs](https://github.com/rust-lang/rfcs/blob/master/text/2592-futures.md#rationale-drawbacks-and-alternatives-to-the-wakeup-design-waker)
to using reference counting but I think it's the best solution for multithreaded executors. Ours is a
single-threaded executor but for the sake of learning, I am still using a reference counted waker design.

### Task state - 04/01/2022

In both `async-task` and `tokio`, task state is stored as an `AtomicUsize`. This allows access to be
synchronized. Given this, they've encoded the state using bitmasks. In my design, I don't need this
and could implement the same thing with an enum. I was planning on opting for this design but I have
little experience with bitmasks (and will need it when working with epoll) so once again, I might as
well learn.

#### Update - 04/01/2022

Turns out it's pretty straightforward. All we do is have certain bits represent a value. If we want
to check if a bit is set, we AND the bit we care about with the state. This will evaluate to 0 for
all bits *not* of interest. For the bit of interest, if it is set, we get back 1 and if not 0.
If we want to check or set bits, we OR the respective bitmasks. Since all the irrelevant bits will
be set to 0, it won't change what is currently set.
