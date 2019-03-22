use std::sync::Arc;
use std::sync::atomic::AtomicPtr;
use crate::cas_utils::c_cas::{CCasPtr};
use crate::cas_utils::Status;

pub struct MCasDesc<T> {
    inner: Arc<Vec<CCasPtr<T>>>,
    expect: Vec<*mut T>,
    new: Vec<*mut T>,
    status: Arc<AtomicPtr<Status>>,
}

pub trait MCas<T> {
    fn m_cas(&self, expect: Vec<*mut T>, new: Vec<*mut T>) ;
}

impl<T> MCas<T> for Arc<Vec<CCasPtr<T>>> {
    fn m_cas(&self, expect: Vec<*mut T>, new: Vec<*mut T>) {
        let start_status = Box::new(Status::Undecided);
        let mut desc = MCasDesc::<T> {
            inner: self.clone(),
            expect,
            new,
            status: Arc::new(AtomicPtr::new(Box::leak(start_status)))
        };
    }
}

pub trait MCasRead<T> {
    fn read(&self) -> *mut T;
}