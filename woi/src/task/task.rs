use std::ptr::NonNull;

use crate::task::header::Header;

pub(crate) struct Task {
    raw: NonNull<()>
}

impl Task {
    pub fn schedule(self) {
        let ptr = self.raw.as_ptr();
        let header = ptr as *const Header;
        unsafe {
            ((*header).vtable.schedule)(ptr)
        }
    }
}