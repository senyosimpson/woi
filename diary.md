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
