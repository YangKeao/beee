use crate::cas_utils::c_cas::{CCasPtr, CCasUnion};
use crate::cas_utils::Status;
use crate::utils::{AtomicNumLikes, AtomicNumLikesMethods, AtomicPtrAddOn};
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub struct MCasDesc<T> {
    inner: Arc<Vec<SingleCas<T>>>,
    status: Arc<AtomicNumLikes>,
}

impl<T> MCasDesc<T> {
    fn help(&self, desc_ptr: *mut CCasUnion<MCasUnion<T>>) -> bool {
        'iter: for (index, item) in self.inner.iter().enumerate() {
            'retry: loop {
                item.origin
                    .c_cas(item.expect, desc_ptr, self.status.clone());
                unsafe {
                    let c_cas_ptr = (*item.origin.inner.get()).load(Ordering::Relaxed);
                    if std::ptr::eq(c_cas_ptr, desc_ptr) {
                        break 'retry;
                    } else {
                        match &mut *c_cas_ptr {
                            CCasUnion::Value(v) => match v {
                                MCasUnion::MCasDesc(v) => {
                                    v.help(c_cas_ptr);
                                }
                                _ => {
                                    self.status.compare_and_swap(
                                        Status::Undecided,
                                        Status::Failed,
                                        Ordering::SeqCst,
                                    );
                                    break 'iter;
                                }
                            },
                            CCasUnion::CCasDesc(c_desc) => {
                                c_desc.help(c_cas_ptr);
                            }
                        }
                    }
                }
            }
            if index == self.inner.len() - 1 {
                self.status.compare_and_swap(
                    Status::Undecided,
                    Status::Successful,
                    Ordering::SeqCst,
                );
            }
        }

        let cond: Status = self.status.get(Ordering::Relaxed);
        let success = cond == Status::Successful;
        for item in self.inner.iter() {
            unsafe {
                (*item.origin.inner.get()).compare_and_swap(
                    desc_ptr,
                    if success { item.new } else { item.expect },
                    Ordering::SeqCst,
                );
            }
        }
        return success;
    }
}

pub enum MCasUnion<T> {
    MCasDesc(MCasDesc<T>),
    Value(T),
}

pub trait MCas<T> {
    fn m_cas(&self) -> bool;
}

pub struct SingleCas<T> {
    origin: CCasPtr<MCasUnion<T>>,
    expect: *mut CCasUnion<MCasUnion<T>>,
    new: *mut CCasUnion<MCasUnion<T>>,
}

impl<T> Clone for SingleCas<T> {
    fn clone(&self) -> Self {
        SingleCas::<T> {
            origin: self.origin.clone(),
            expect: self.expect.clone(),
            new: self.new.clone(),
        }
    }
}

impl<T> Ord for SingleCas<T> {
    fn cmp(&self, other: &SingleCas<T>) -> std::cmp::Ordering {
        unsafe {
            self.origin
                .inner
                .get_addr()
                .cmp(&other.origin.inner.get_addr())
        }
    }
}

impl<T> PartialOrd for SingleCas<T> {
    fn partial_cmp(&self, other: &SingleCas<T>) -> Option<std::cmp::Ordering> {
        unsafe {
            Some(
                self.origin
                    .inner
                    .get_addr()
                    .cmp(&other.origin.inner.get_addr()),
            )
        }
    }
}

impl<T> PartialEq for SingleCas<T> {
    fn eq(&self, other: &SingleCas<T>) -> bool {
        unsafe {
            self.origin
                .inner
                .get_addr()
                .eq(&other.origin.inner.get_addr())
        }
    }
}

impl<T> Eq for SingleCas<T> {}

impl<T> MCas<T> for Vec<SingleCas<T>> {
    fn m_cas(&self) -> bool {
        let mut sort_self: Vec<SingleCas<T>> = self.clone();
        sort_self.sort();

        let mut desc = CCasUnion::Value(MCasUnion::MCasDesc(MCasDesc::<T> {
            inner: Arc::new(sort_self),
            status: Arc::new(AtomicNumLikes::new(Status::Undecided)),
        }));
        let desc_ptr = &mut desc as *mut CCasUnion<MCasUnion<T>>;

        match desc {
            CCasUnion::Value(v) => match v {
                MCasUnion::MCasDesc(v) => v.help(desc_ptr),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}

pub trait MCasRead<T> {
    fn read(&self) -> *mut T;
}

impl<T> MCasRead<T> for CCasPtr<MCasUnion<T>> {
    fn read(&self) -> *mut T {
        loop {
            let c_cas_ptr = self.load();
            let c_union_ptr = unsafe { (*self.inner.get()).load(Ordering::Relaxed) };
            unsafe {
                match &mut *c_cas_ptr {
                    MCasUnion::MCasDesc(desc) => {
                        desc.help(c_union_ptr);
                    }
                    MCasUnion::Value(v) => {
                        return v as *mut T;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn single_thread_m_cas() {
        let mut num1 = CCasUnion::Value(MCasUnion::Value(1));
        let  num1_ptr = &mut num1 as *mut CCasUnion<MCasUnion<i32>>;
        let mut num2 = CCasUnion::Value(MCasUnion::Value(2));
        let  num2_ptr = &mut num2 as *mut CCasUnion<MCasUnion<i32>>;
        let mut num3 = CCasUnion::Value(MCasUnion::Value(3));
        let  num3_ptr = &mut num3 as *mut CCasUnion<MCasUnion<i32>>;
        let mut num4 = CCasUnion::Value(MCasUnion::Value(4));
        let  num4_ptr = &mut num4 as *mut CCasUnion<MCasUnion<i32>>;

        let c_cas_ptr_origin1 = CCasPtr::from_c_cas_union(num1_ptr);
        let c_cas_ptr_origin3 = CCasPtr::from_c_cas_union(num3_ptr);
        let first_cas = SingleCas {
            origin: c_cas_ptr_origin1.clone(),
            expect: num2_ptr,
            new: num2_ptr
        };
        let second_cas = SingleCas {
            origin: c_cas_ptr_origin3.clone(),
            expect: num3_ptr,
            new: num4_ptr
        };

        let m_cas = vec![first_cas, second_cas];
        m_cas.m_cas();
        assert_eq!(unsafe {*c_cas_ptr_origin1.read()}, 1);
        assert_eq!(unsafe {*c_cas_ptr_origin3.read()}, 3);

        let first_cas = SingleCas {
            origin: c_cas_ptr_origin1.clone(),
            expect: num1_ptr,
            new: num2_ptr
        };
        let second_cas = SingleCas {
            origin: c_cas_ptr_origin3.clone(),
            expect: num3_ptr,
            new: num4_ptr
        };
        let m_cas = vec![first_cas, second_cas];
        m_cas.m_cas();
        assert_eq!(unsafe {*c_cas_ptr_origin1.read()}, 2);
        assert_eq!(unsafe {*c_cas_ptr_origin3.read()}, 4);
    }
}