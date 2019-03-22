use crate::cas_utils::c_cas::{CCasPtr, CCasUnion};
use crate::cas_utils::Status;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use crate::utils::{AtomicNumLikes, AtomicNumLikesMethods};

pub struct MCasDesc<T> {
    inner: Arc<Vec<CCasPtr<MCasUnion<T>>>>,
    expect: Vec<*mut CCasUnion<MCasUnion<T>>>,
    new: Vec<*mut CCasUnion<MCasUnion<T>>>,
    status: Arc<AtomicNumLikes>,
}

impl<T> MCasDesc<T> {
    fn help(&self, desc_ptr: *mut CCasUnion<MCasUnion<T>>) -> bool {
        'iter: for (index, item) in self.inner.iter().enumerate() {
            'retry: loop {
                item.c_cas(self.expect[index], desc_ptr, self.status.clone());
                unsafe {
                    let c_cas_ptr = item.inner.load(Ordering::Relaxed);
                    if std::ptr::eq(c_cas_ptr, desc_ptr) {
                        break 'retry;
                    } else {
                        match &mut *c_cas_ptr {
                            CCasUnion::Value(v) => match v {
                                MCasUnion::CCasDesc(v) => {
                                    v.help(c_cas_ptr);
                                }
                                _ => {
                                    self.status.compare_and_swap(Status::Undecided, Status::Failed, Ordering::SeqCst);
                                    break 'iter;
                                }
                            },
                            _ => unimplemented!(), // TODO: Maybe we need to help CCAS
                        }
                    }
                }
            }
            if index == self.inner.len() - 1 {
                self.status.compare_and_swap(Status::Undecided, Status::Successful, Ordering::SeqCst);
            }
        }

        let cond: Status = self.status.get(Ordering::Relaxed);
        let success = cond == Status::Successful;
        for (index, item) in self.inner.iter().enumerate() {
            item.inner.compare_and_swap(desc_ptr, if success {self.new[index]} else {self.expect[index]}, Ordering::SeqCst);
        }
        return success;
    }
}

pub enum MCasUnion<T> {
    CCasDesc(MCasDesc<T>),
    Value(T),
}

pub trait MCas<T> {
    fn m_cas(
        &self,
        expect: Vec<*mut CCasUnion<MCasUnion<T>>>,
        new: Vec<*mut CCasUnion<MCasUnion<T>>>,
    );
}

impl<T> MCas<T> for Arc<Vec<CCasPtr<MCasUnion<T>>>> {
    fn m_cas(
        &self,
        expect: Vec<*mut CCasUnion<MCasUnion<T>>>,
        new: Vec<*mut CCasUnion<MCasUnion<T>>>,
    ) {
        let mut desc = MCasDesc::<T> {
            inner: self.clone(),
            expect,
            new,
            status: Arc::new(AtomicNumLikes::new(Status::Undecided)),
        };
    }
}

pub trait MCasRead<T> {
    fn read(&self) -> *mut T;
}
