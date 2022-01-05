use std::{
    alloc::{self, Layout},
    future::Future,
    mem,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use crate::task::{header::Header, state::State, task::Task};

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
                state: State::new(),
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
    // is for: the future, the scheduling function and the task header
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

    pub unsafe fn dealloc(ptr: *const()) {
        let layout = Self::layout();
        // TODO: Investigate if I need to use .drop_in_place()
        alloc::dealloc(ptr as *mut u8, layout.layout);
    }

    // Makes a clone of the waker
    // Increments the number of references to the waker
    unsafe fn clone_waker(ptr: *const ()) -> RawWaker {
        let raw = Self::from_ptr(ptr);
        let header = &mut *(raw.header as *mut Header); 
        header.state.ref_incr();
        RawWaker::new(ptr, &Self::RAW_WAKER_VTABLE)
    }

    // This is responsible for decrementing a reference count and ensuring
    // the task is destroyed if the reference count is 0
    unsafe fn drop_waker(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);
        let header = &mut *(raw.header as *mut Header); 
        header.state.ref_decr();
        if header.state.ref_count() == 0 {
            Self::dealloc(ptr)
        }
    }

    // Wakes the task
    // One requirement here is that it must be safe
    // to call `wake` even if the task has been driven to completion
    unsafe fn wake(ptr: *const ()) {
        // Here the caller gives us a reference count. If there is no
        // need to schedule the task then we consume the reference count

        // TODO: We need to hold a reference count if we have to schedule
        // the task otherwise we will cause UB. This is likely to require
        // us to have to keep the state of the task and only decrement the
        // waker if we do not need to schedule it to run again
        Self::schedule(ptr);
        Self::drop_waker(ptr);
    }

    unsafe fn wake_by_ref(ptr: *const ()) {
        Self::schedule(ptr);
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
        let header = &mut *(raw.header as *mut Header);

        let waker = Waker::from_raw(RawWaker::new(ptr, &Self::RAW_WAKER_VTABLE));
        let cx = &mut Context::from_waker(&waker);

        let status = &mut *raw.status;
        // TODO: Improve error handling
        let future = match status {
            Status::Running(future) => future,
            _ => panic!("Wrong stage"),
        };

        // Safety: The future is allocated on the heap and therefore we know
        // it has a stable memory address
        // NOTE: Not sure how to phrase this. We don't need to use crate::pin! here
        // because we already have a mutable reference to the future
        let future = Pin::new_unchecked(future);
        match future.poll(cx) {
            Poll::Ready(out) => {
                header.state.transition_to_complete();
                *raw.status = Status::Finished(out)
            }
            Poll::Pending => {
                // Schedule again in future
                // (header.vtable.schedule)(ptr);
            }
        }
    }

    unsafe fn get_output(ptr: *const ()) -> *const () {
        let raw = Self::from_ptr(ptr);
        // TODO: Improve error handling
        match mem::replace(&mut *raw.status, Status::Consumed) {
            Status::Finished(output) => &output as *const _ as *const (),
            _ => panic!("Could not retrieve output!"),
        }
    }
}
