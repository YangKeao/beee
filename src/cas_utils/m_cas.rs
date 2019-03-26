use crate::cas_utils::c_cas::{CCasPtr, CCasUnion};
use crate::cas_utils::Status;
use crate::utils::{AtomicNumLikes, AtomicNumLikesMethods};
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
                item.origin.c_cas(item.expect, desc_ptr, self.status.clone());
                unsafe {
                    let c_cas_ptr = item.origin.load(Ordering::Relaxed);
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
            item.origin.compare_and_swap(
                desc_ptr,
                if success { item.new } else { item.expect },
                Ordering::SeqCst,
            );
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
impl<T> SingleCas<T> {
    pub fn new(
        origin: &AtomicMCasPtr<T>,
        expect: *mut MCasPtr<T>,
        new: *mut MCasPtr<T>,
    ) -> SingleCas<T> {
        Self {
            origin: origin.inner.clone(),
            expect: expect as *mut CCasUnion<MCasUnion<T>>,
            new: new as *mut CCasUnion<MCasUnion<T>>,
        }
    }
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
        self.origin.get_addr().cmp(&other.origin.get_addr())
    }
}

impl<T> PartialOrd for SingleCas<T> {
    fn partial_cmp(&self, other: &SingleCas<T>) -> Option<std::cmp::Ordering> {
        Some(self.origin.get_addr().cmp(&other.origin.get_addr()))
    }
}

impl<T> PartialEq for SingleCas<T> {
    fn eq(&self, other: &SingleCas<T>) -> bool {
        self.origin.get_addr().eq(&other.origin.get_addr())
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

pub struct AtomicMCasPtr<T> {
    inner: CCasPtr<MCasUnion<T>>,
}
impl<T> Clone for AtomicMCasPtr<T> {
    fn clone(&self) -> Self {
        AtomicMCasPtr {
            inner: self.inner.clone(),
        }
    }
}

impl<T> AtomicMCasPtr<T> {
    pub fn new(ptr: &mut MCasPtr<T>) -> Self {
        AtomicMCasPtr {
            inner: CCasPtr::from_c_cas_union(ptr.get_mut_ptr()),
        }
    }
    pub fn read(&self) -> &mut T {
        loop {
            let c_union_ptr = self.inner.load(Ordering::Relaxed);
            let c_cas_ptr = unsafe { (*c_union_ptr).load() };
            unsafe {
                match &mut *c_cas_ptr {
                    MCasUnion::MCasDesc(desc) => {
                        desc.help(c_union_ptr);
                    }
                    MCasUnion::Value(v) => {
                        return v;
                    }
                }
            }
        }
    }
    pub fn get_m_cas_ptr(&self, order: Ordering) -> *mut MCasPtr<T> {
        self.inner.load(order) as *mut MCasPtr<T>
    }
}

pub struct MCasPtr<T> {
    inner: CCasUnion<MCasUnion<T>>,
}
impl<T> MCasPtr<T> {
    pub fn new(val: T) -> MCasPtr<T> {
        Self {
            inner: CCasUnion::Value(MCasUnion::Value(val)),
        }
    }
    pub fn read_mut(&mut self) -> *mut T {
        let c_cas_union = &mut self.inner;
        let c_cas_union_ptr = c_cas_union as *mut CCasUnion<MCasUnion<T>>;
        loop {
            match c_cas_union {
                CCasUnion::Value(v) => match v {
                    MCasUnion::Value(v) => return v as *mut T,
                    MCasUnion::MCasDesc(desc) => {
                        desc.help(c_cas_union_ptr);
                    }
                },
                CCasUnion::CCasDesc(desc) => {
                    desc.help(c_cas_union_ptr);
                }
            }
        }
    }
    pub fn get_mut_ptr(&mut self) -> *mut CCasUnion<MCasUnion<T>> {
       &mut self.inner as *mut CCasUnion<MCasUnion<T>>
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn single_thread_m_cas() {
        let mut num1 = MCasPtr::new(1);
        let num1_ptr = num1.get_mut_ptr() as *mut MCasPtr<i32>;
        let mut num2 = MCasPtr::new(2);
        let num2_ptr = num2.get_mut_ptr() as *mut MCasPtr<i32>;
        let mut num3 = MCasPtr::new(3);
        let num3_ptr = num3.get_mut_ptr() as *mut MCasPtr<i32>;
        let mut num4 = MCasPtr::new(4);
        let num4_ptr = num4.get_mut_ptr() as *mut MCasPtr<i32>;

        let atomic_num1 = AtomicMCasPtr::new(&mut num1);
        let first_cas = SingleCas::new(&atomic_num1.clone(), num2_ptr, num2_ptr);

        let atomic_num3 = AtomicMCasPtr::new(&mut num3);
        let second_cas = SingleCas::new(&atomic_num3.clone(), num3_ptr, num4_ptr);

        let m_cas = vec![first_cas, second_cas];
        assert_eq!(m_cas.m_cas(), false);
        assert_eq!(*atomic_num1.read(), 1);
        assert_eq!(*atomic_num3.read(), 3);

        let first_cas = SingleCas::new(&atomic_num1.clone(), num1_ptr, num2_ptr);
        let second_cas = SingleCas::new(&atomic_num3.clone(), num3_ptr, num4_ptr);
        let m_cas = vec![first_cas, second_cas];
        assert_eq!(m_cas.m_cas(), true);
        assert_eq!(*atomic_num1.read(), 2);
        assert_eq!(*atomic_num3.read(), 4);
    }
}
