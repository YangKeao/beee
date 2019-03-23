use crate::cas_utils::Status;
use crate::utils::{AtomicNumLikes, AtomicNumLikesMethods};
use std::cell::UnsafeCell;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub struct CCasDesc<T> {
    inner: Arc<UnsafeCell<AtomicPtr<CCasUnion<T>>>>,
    expect: *mut CCasUnion<T>,
    new: *mut CCasUnion<T>,
    cond: Arc<AtomicNumLikes>,
}

impl<T> CCasDesc<T> {
    pub fn help(&self, desc_ptr: *mut CCasUnion<T>) {
        let cond: Status = self.cond.get(Ordering::Relaxed);
        let success = cond == Status::Undecided;
        unsafe {
            (*self.inner.get()).compare_and_swap(
                desc_ptr,
                if success { self.new } else { self.expect },
                Ordering::SeqCst,
            ); // TODO: set order carefully
        }
    }
}

pub enum CCasUnion<T> {
    CCasDesc(CCasDesc<T>),
    Value(T),
}

impl<T> CCasUnion<T> {
    pub fn borrow_mut_c_cas_desc(&mut self) -> &mut CCasDesc<T> {
        match self {
            CCasUnion::CCasDesc(c_cas_desc) => c_cas_desc,
            _ => unreachable!(),
        }
    }
}

pub struct CCasPtr<T> {
    pub inner: Arc<UnsafeCell<AtomicPtr<CCasUnion<T>>>>,
}

impl<T> Clone for CCasPtr<T> {
    fn clone(&self) -> Self {
        CCasPtr::<T> {
            inner: self.inner.clone(),
        }
    }
}

impl<T> CCasPtr<T> {
    pub fn from_value(val: T) -> CCasPtr<T> {
        CCasPtr::<T> {
            inner: Arc::new(UnsafeCell::new(AtomicPtr::new(Box::leak(Box::new(CCasUnion::Value(val))))))
        }
    }
    pub fn from_c_cas_union(union: *mut CCasUnion<T>) -> CCasPtr<T> {
        CCasPtr::<T> {
            inner: Arc::new(UnsafeCell::new(AtomicPtr::new(union)))
        }
    }
    pub fn c_cas(
        &self,
        expect: *mut CCasUnion<T>,
        new: *mut CCasUnion<T>,
        cond: Arc<AtomicNumLikes>,
    ) {
        let mut desc = CCasUnion::CCasDesc(CCasDesc::<T> {
            inner: self.inner.clone(),
            expect,
            new,
            cond: cond.clone(),
        });

        let expect_ptr = desc.borrow_mut_c_cas_desc().expect;
        let desc_ptr = &mut desc as *mut CCasUnion<T>;

        loop {
            unsafe {
                let res = (*desc.borrow_mut_c_cas_desc().inner.get()).compare_and_swap(
                    expect_ptr,
                    desc_ptr,
                    Ordering::SeqCst,
                ); // TODO: set order carefully
                if std::ptr::eq(res, expect_ptr) {
                    desc.borrow_mut_c_cas_desc().help(desc_ptr);
                    break;
                } else {
                    match &*res {
                        CCasUnion::CCasDesc(c_cas_desc) => c_cas_desc.help(desc_ptr),
                        _ => return, // TODO: mark failed
                    }
                }
            }
        }
    }

    pub fn load(&self) -> *mut T {
        loop {
            unsafe {
                let v = (*self.inner.get()).load(Ordering::SeqCst); // TODO: set order carefully
                match &mut *v {
                    CCasUnion::CCasDesc(c_cas_desc) => c_cas_desc.help(v),
                    CCasUnion::Value(val) => return val as *mut T,
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_single_c_cas() {
        let success = Arc::new(AtomicNumLikes::new(Status::Successful));
        let undecided = Arc::new(AtomicNumLikes::new(Status::Undecided));

        let mut num = CCasUnion::Value(1);
        let num_ptr = &mut num as *mut CCasUnion<i32>;

        let mut num2 =CCasUnion::Value(2);
        let num2_ptr = &mut num2 as *mut CCasUnion<i32>;

        let c_cas_ptr = CCasPtr::from_c_cas_union(num_ptr);

        c_cas_ptr.c_cas(num_ptr, num2_ptr, success.clone());
        assert_eq!(unsafe {*c_cas_ptr.load()}, 1);
    }
}