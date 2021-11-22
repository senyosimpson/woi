use std::{
    alloc::{self, Layout},
    future::Future,
    mem,
    pin::Pin,
    ptr::NonNull,
    sync::atomic::AtomicUsize,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use crate::task::header::Header;
use crate::task::task::Task;

pub(crate) trait Schedule {
    fn schedule(&self, task: Task);
}

pub struct TaskVTable {
    pub(crate) poll: unsafe fn(*const ()),
    pub(crate) get_output: unsafe fn(*const ()) -> *const (),
    pub(crate) schedule: unsafe fn(*const ()),
}

// The status of a future. This contains either the future
// itself or the output of the future
pub enum Status<F: Future> {
    Running(F),
    Finished(F::Output),
    Consumed,
}

// Memory layout of a task
pub struct TaskLayout {
    layout: Layout,
    offset_schedule: usize,
    offset_status: usize,
}

// Having the C representation means we are guaranteed
// on the memory layout of the task
#[repr(C)]
pub(crate) struct RawTask<F: Future, S> {
    pub(crate) header: *const Header,
    pub(crate) scheduler: *const S,
    pub(crate) status: *mut Status<F>,
}

impl<F, S> RawTask<F, S>
where
    F: Future,
    S: Schedule,
{
    // What implication is there for having a const within an impl? Is that the same
    // as having it outside?
    const RAW_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
        Self::clone_waker,
        Self::wake,
        Self::wake_by_ref,
        Self::drop_waker,
    );


    pub fn new(future: F, scheduler: S) -> NonNull<()> {
        let task_layout = Self::layout();
        unsafe {
            let ptr = match NonNull::new(alloc::alloc(task_layout.layout) as *mut ()) {
                None => panic!("Could not allocate task!"),
                Some(ptr) => ptr,
            };

            let raw = Self::from_ptr(ptr.as_ptr());

            let header = Header {
                // state: AtomicUsize::new(0), // Todo: Understand the role of the state
                state: 0,
                vtable: &TaskVTable {
                    poll: Self::poll,
                    get_output: Self::get_output,
                    schedule: Self::schedule,
                },
            };
            (raw.header as *mut Header).write(header);
            (raw.scheduler as *mut S).write(scheduler);

            let status = Status::Running(future);
            raw.status.write(status);

            ptr
        }
    }

    fn from_ptr(ptr: *const ()) -> Self {
        let task_layout = Self::layout();
        let ptr = ptr as *const u8;
        unsafe {
            Self {
                header: ptr as *const Header,
                scheduler: ptr.add(task_layout.offset_schedule) as *const S,
                status: ptr.add(task_layout.offset_status) as *mut Status<F>,
            }
        }
    }

    // Calculates the memory layout requirements and stores offsets into the
    // task to find the respective fields. The space that needs to be allocated
    // are for: the future, the scheduling function and the task header
    pub fn layout() -> TaskLayout {
        let header_layout = Layout::new::<Header>();
        let schedule_layout = Layout::new::<S>();
        let stage_layout = Layout::new::<Status<F>>();

        let layout = header_layout;
        let (layout, offset_schedule) = layout
            .extend(schedule_layout)
            .expect("Could not allocate task!");
        let (layout, offset_status) = layout
            .extend(stage_layout)
            .expect("Could not allocate task!");

        TaskLayout {
            layout,
            offset_schedule,
            offset_status,
        }
    }

    // Makes a clone of the waker
    // Increments the number of references to the waker. Why this
    // is necessary is yet to be seen
    unsafe fn clone_waker(ptr: *const ()) -> RawWaker {
        let raw = Self::from_ptr(ptr);
        RawWaker::new(ptr, &Self::RAW_WAKER_VTABLE)
    }

    // This is responsible for decrementing a reference count and ensuring
    // the task is destroyed if the reference count is 0
    unsafe fn drop_waker(ptr: *const ()) {}

    // Wake the task
    unsafe fn wake(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);

        // This is where we would schedule the task onto the executor
    }
    unsafe fn wake_by_ref(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);

        // This is where we would schedule the task onto the executor
    }

    unsafe fn schedule(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);

        let task = Task {
            raw: NonNull::new_unchecked(ptr as *mut ()),
        };

        let scheduler = &*raw.scheduler;
        scheduler.schedule(task)
    }

    // Runs the future and updates its state
    unsafe fn poll(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);

        let waker = Waker::from_raw(RawWaker::new(ptr, &Self::RAW_WAKER_VTABLE));
        let cx = &mut Context::from_waker(&waker);

        let status = &mut *raw.status;

        let future = match status {
            Status::Running(future) => future,
            _ => panic!("Wrong stage"),
        };

        // Should we Box::pin here or is pinning on the stack fine?
        let future = Pin::new_unchecked(future);
        match future.poll(cx) {
            Poll::Ready(v) => {
                let header = &mut *(raw.header as *mut Header);
                header.state = 1;
                *raw.status = Status::Finished(v)
            },
            Poll::Pending => {
                // Schedule again in future
            }
        }
    }

    unsafe fn get_output(ptr: *const ()) -> *const () {
        let raw = Self::from_ptr(ptr);

        // If you can read the output, perform the condition
        // This should check the status of the task
        match mem::replace(&mut *raw.status, Status::Consumed) {
            Status::Finished(output) => &output as *const _ as *const (),
            _ => panic!("Could not retrieve output!"),
        }
    }
}
