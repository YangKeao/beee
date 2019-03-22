use std::sync::Arc;
use std::sync::atomic::AtomicPtr;
use crate::cas_utils::c_cas::{CCasUnion, CCasPtr};
use crate::cas_utils::{Status, UNDECIDED};
use std::sync::atomic::Ordering;

pub struct MCasDesc<T> {
    inner: Arc<Vec<CCasPtr<MCasUnion<T>>>>,
    expect: Vec<*mut T>,
    new: Vec<*mut T>,
    status: Arc<AtomicPtr<Status>>, // TODO: need Atomic Status, but not AtomicPtr
}

impl<T> MCasDesc<T> {
    fn help(&self, desc_ptr: *mut MCasUnion<T>) {
        for (index, item) in self.inner.iter().enumerate() {
            loop {
                item.c_cas(self.expect[index], desc_ptr, self.status.clone());
                unsafe {
                    let v = item.inner.load(Ordering::Relaxed);
                    match &mut *v {
                        CCasUnion::Value(v) => {
                            let v_ptr = v as *mut MCasUnion<T>;
                            if std::ptr::eq(v_ptr, desc_ptr) {
                                break
                            } else {
                                match v {
                                    MCasUnion::CCasDesc(v) => {
                                        v.help(v_ptr);
                                    }
                                    _ => {
                                        self.status.compare_and_swap()
                                    }
                                }
                            }
                        }
                        _ => {
                            unimplemented!()
                        }
                    }
                }
            }
        }
    }
}

pub enum MCasUnion<T> {
    CCasDesc(MCasDesc<T>),
    Value(T)
}

pub trait MCas<T> {
    fn m_cas(&self, expect: Vec<*mut T>, new: Vec<*mut T>) ;
}

impl<T> MCas<T> for Arc<Vec<CCasPtr<MCasUnion<T>>>> {
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