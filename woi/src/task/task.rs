use std::ptr::NonNull;

use crate::task::header::Header;

pub(crate) struct Task {
    pub(crate) raw: NonNull<()>
}

impl Task {
    pub fn schedule(self) {
        let ptr = self.raw.as_ptr();
        let header = ptr as *const Header;
        unsafe {
            ((*header).vtable.schedule)(ptr)
        }
    }
    
    pub fn run(self) {
        let ptr = self.raw.as_ptr();
        let header = ptr as *const Header;
        unsafe {
            ((*header).vtable.poll)(ptr)
        }
    }
}