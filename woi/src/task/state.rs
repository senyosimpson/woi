#[derive(PartialEq, Eq)]
pub(crate) enum Status {
    Running,
    Done,
}

pub(crate) struct State {
    pub(crate) status: Status,
    pub(crate) ref_count: usize,
}

impl State {
    pub fn new() -> State {
        State {
            status: Status::Running,
            ref_count: 1,
        }
    }

    pub fn ref_incr(&mut self) {
        self.ref_count += 1;
    }

    pub fn ref_decr(&mut self) {
        self.ref_count -= 1;
    }

    pub fn transition_to_done(&mut self) {
       self.status = Status::Done;
    }
}
