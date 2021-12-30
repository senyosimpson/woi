use crate::task::state::State;
use crate::task::raw::TaskVTable;



pub(crate) struct Header {
    pub state: State,
    pub vtable: &'static TaskVTable, // Why &'static? Think cause they are fns
}
