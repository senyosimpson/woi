use std::sync::atomic::AtomicUsize;

struct State(AtomicUsize);


impl State {
    fn new() -> State {
        State(AtomicUsize::new(0))
    }
}