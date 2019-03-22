use std::sync::atomic::AtomicI32;

pub static GLOBAL_EPOCH: AtomicI32 = AtomicI32::new(0);

thread_local! {
    pub(crate) static thread_status: RefCell<ThreadStatus> = RefCell::new(ThreadStatus::new());
}

struct ThreadStatus {
    pub(crate) retired_list: Vec<Box<Fn()>>
}

struct Guard {}

impl ThreadStatus {
    pub fn new() -> ThreadStatus {
        return ThreadStatus {
            retired_list: Vec::new()
        }
    }

    pub fn pin() -> Guard {
        return Guard {}
    }
}