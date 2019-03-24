//! # Usage
//!
//! We need to use `CCasUnion` to `read` and `c_cas`. All directly access of the pointer in
//! `CCasUnion` may cause undefined behavior.
//!
//! Maybe in the future we will provide some way to store `*mut T` more convinent. Becasue `*mut T`
//! doesn't impl `Send` trait. But in this situation we frequently need to send pointer to multiple
//! threads.
//!
//! ```
//! # use beee::cas_utils::c_cas::*;
//! # use beee::cas_utils::*;
//! # use beee::utils::{AtomicNumLikes, AtomicNumLikesMethods};
//! # use std::sync::Arc;
//!
//! let success = Arc::new(AtomicNumLikes::new(Status::Successful));
//! let undecided = Arc::new(AtomicNumLikes::new(Status::Undecided));
//!
//! let mut num = CCasUnion::Value(1);
//! let num_ptr = &mut num as *mut CCasUnion<i32>;
//! let mut num2 =CCasUnion::Value(2);
//! let num2_ptr = &mut num2 as *mut CCasUnion<i32>;
//!
//! let c_cas_ptr = CCasPtr::from_c_cas_union(num_ptr);
//! c_cas_ptr.c_cas(num_ptr, num2_ptr, success.clone()); // This cas will not happen because of `Status::Successful`
//! assert_eq!(unsafe {*c_cas_ptr.load()}, 1);
//!
//! c_cas_ptr.c_cas(num_ptr, num2_ptr, undecided.clone()); // This will cas values because we pass `Status::Undecided`
//! assert_eq!(unsafe {*c_cas_ptr.load()}, 2);
//! ```
//!
//! # Notes
//!
//! The detail algorithm is written in [Practicallock-freedom](https://www.cl.cam.ac.uk/techreports/UCAM-CL-TR-579.pdf).

use crate::cas_utils::Status;
use crate::utils::{AtomicNumLikes, AtomicNumLikesMethods};
use std::cell::UnsafeCell;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// A CCas Descriptor
///
/// # Fields
///
/// * `inner`: Store original AtomicPtr
/// * `expect`: Expected value of inner
/// * `new`: New value of inner
/// * `cond`: Cond var. Only if it equals Status::Undecided will cas happens
pub struct CCasDesc<T> {
    inner: Arc<UnsafeCell<AtomicPtr<CCasUnion<T>>>>, // TODO: UnsafeCell is only for get_addr, maybe there is a better way to get addr rather than use UnsafeCell?
    expect: *mut CCasUnion<T>,
    new: *mut CCasUnion<T>,
    cond: Arc<AtomicNumLikes>,
}

impl<T> CCasDesc<T> {
    /// Help to run CCAS
    ///
    /// # Arguments
    ///
    /// * `desc_ptr`: Address of desc_ptr which was cas into inner at `CCasPtr::c_cas` function
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

/// Union Type of CCasDesc and True Value
///
/// # Variants
///
/// * `CCasDesc`: Used to store CCasDesc
/// * `Value`: Used to store Origin and Expect Value
pub enum CCasUnion<T> {
    CCasDesc(CCasDesc<T>),
    Value(T),
}

impl<T> CCasUnion<T> {
    /// Get a `CCasDesc` from `CCasUnion`. If it's not a `CCasUnion`, **`unreachable!()` will make the process panic!**
    pub fn borrow_mut_c_cas_desc(&mut self) -> &mut CCasDesc<T> {
        match self {
            CCasUnion::CCasDesc(c_cas_desc) => c_cas_desc,
            _ => unreachable!(),
        }
    }
}

/// Only an alias of `Arc<UnsafeCell<AtomicPtr<CCasUnion<T>>>>`
pub struct CCasPtr<T> {
    pub inner: Arc<UnsafeCell<AtomicPtr<CCasUnion<T>>>>,
}
unsafe impl<T> std::marker::Send for CCasPtr<T> {}

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
    use std::thread;

    const THREAD_NUM: usize = 100;
    const ITER_NUM: usize = 10000;

    // TODO: should we provide some better api to handle ptr across different thread?
    struct SendPtr<T> {
        ptr: *mut T
    }
    unsafe impl<T> Send for SendPtr<T> {}
    impl<T> Clone for SendPtr<T> {
        fn clone(&self) -> Self {
            SendPtr::<T> {
                ptr: self.ptr.clone()
            }
        }
    }
    impl<T> SendPtr<T> {
        fn new(ptr: *mut T) -> Self {
            SendPtr::<T> {
                ptr,
            }
        }
        fn get_ptr(&self) -> *mut T {
            self.ptr
        }
    }

    #[test]
    fn multi_thread_test() {
        let success = Arc::new(AtomicNumLikes::new(Status::Successful));
        let undecided = Arc::new(AtomicNumLikes::new(Status::Undecided));

        let mut num = CCasUnion::Value(1);
        let num_ptr = SendPtr::new(&mut num as *mut CCasUnion<i32>);

        let mut num2 =CCasUnion::Value(2);
        let num2_ptr = SendPtr::new(&mut num2 as *mut CCasUnion<i32>);

        let c_cas_ptr = CCasPtr::from_c_cas_union(num_ptr.get_ptr());
        let write_threads = (0..THREAD_NUM).map(|_| {
            let c_cas_ptr = c_cas_ptr.clone();
            let num_ptr = num_ptr.clone();
            let num2_ptr = num2_ptr.clone();
            let success = success.clone();
            let undecided = undecided.clone();
            thread::spawn(move || {
                for _ in 0..ITER_NUM {
                    c_cas_ptr.c_cas(num_ptr.get_ptr(), num2_ptr.get_ptr(), success.clone());
                    c_cas_ptr.c_cas(num_ptr.get_ptr(), num2_ptr.get_ptr(), undecided.clone());
                    c_cas_ptr.c_cas(num_ptr.get_ptr(), num2_ptr.get_ptr(), undecided.clone());
                    c_cas_ptr.c_cas(num_ptr.get_ptr(), num2_ptr.get_ptr(), success.clone());
                }
            })
        });
        let read_threads = (0..THREAD_NUM).map(|_| {
            let c_cas_ptr = c_cas_ptr.clone();
            thread::spawn(move || {
                for _ in 0..ITER_NUM {
                    let num = unsafe { *c_cas_ptr.load() };
                    assert!(num == 1 || num == 2);
                }
            })
        });

        for t in write_threads {
            t.join().unwrap();
        }
        for t in read_threads {
            t.join().unwrap();
        }
    }
}