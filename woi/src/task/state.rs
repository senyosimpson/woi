// The task has been scheduled onto the executor
const SCHEDULED: usize = 1 << 0;

// The task is currently being run
const RUNNING: usize = 1 << 1;

// The task is complete
const COMPLETE: usize = 1 << 2;

// The idea of using a state mask and ref count mask and figuring
// out how much to shift is from Tokio
const STATE_MASK: usize = SCHEDULED | RUNNING | COMPLETE;

// The bits belonging to the ref count. These are the upper bits.
// It is calculated by inverting the bits belonging to the
// state i.e 0011 -> 1100
const REF_COUNT_MASK: usize = !STATE_MASK;

// TODO: Word explanation better
// This calculates how many 0s there are in the binary number. This
// takes advantage of the structure of the REF_COUNT_MASK to figure
// out how many bits to shift to the left to get to the reference.
// Since we will *always* a number starting with 1s and ending in 0s
// we can figure this out i.e 111000 for a ref count mask means we
// need to shift left 3 times to get to the ref count bits
const REF_COUNT_SHIFT: usize = REF_COUNT_MASK.count_zeros() as usize;

const REF_ONE: usize = 1 << REF_COUNT_SHIFT;

const INITIAL_STATE: usize = (REF_ONE * 2) | SCHEDULED;

pub(crate) struct State {
    pub(crate) state: usize,
}

impl State {
    pub fn new() -> State {
        State { state: INITIAL_STATE }
    }

    pub fn ref_incr(&mut self) {
        self.state += REF_ONE;
    }

    pub fn ref_decr(&mut self) {
        self.state -= REF_ONE;
    }

    pub fn ref_count(&self) -> usize {
        // To calculate the ref count, we AND with the ref count mask
        // and then shift the bits down so that they begin at the
        // start bit of the reference count
        (self.state & REF_COUNT_MASK) >> REF_COUNT_SHIFT
    }

    pub fn is_complete(&self) -> bool {
        self.state & COMPLETE == COMPLETE
    }

    pub fn transition_to_complete(&mut self) {
        self.state |= COMPLETE;
    }

    pub fn transition_to_running(&mut self) {
        self.state |= RUNNING;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn init_ref_count_ok() {
        let state = State::new();
        assert_eq!(state.ref_count(), 2);
    }

    #[test]
    fn incr_ref_count_ok() {
        let mut state = State::new();
        state.ref_incr();
        assert_eq!(state.ref_count(), 3);
    }
    
    #[test]
    fn decr_ref_count_ok() {
        let mut state = State::new();
        state.ref_decr();
        assert_eq!(state.ref_count(), 1);
    }
}
