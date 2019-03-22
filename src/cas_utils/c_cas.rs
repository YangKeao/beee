use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering;
use crate::cas_utils::Status;
use std::sync::Arc;

pub struct CCasDesc<T> {
    inner: Arc<AtomicPtr<CCasUnion<T>>>,
    expect: *mut CCasUnion<T>,
    new: *mut CCasUnion<T>,
    cond: Arc<AtomicPtr<Status>>,
}

impl<T> CCasDesc<T> {
    pub fn help(&self, desc_ptr: *mut CCasUnion<T>) {
        let success = unsafe {*self.cond.load(Ordering::Relaxed) == Status::Undecided};
        self.inner.compare_and_swap(desc_ptr, if success {self.new} else {self.expect}, Ordering::SeqCst); // TODO: set order carefully
    }
}

pub enum CCasUnion<T> {
    CCasDesc(CCasDesc<T>),
    Value(T)
}

impl<T> CCasUnion<T> {
    pub fn borrow_mut_c_cas_desc(&mut self) -> &mut CCasDesc<T> {
        match self {
            CCasUnion::CCasDesc(c_cas_desc) => c_cas_desc,
            _ => unreachable!()
        }
    }
}

pub struct CCasPtr<T> {
    pub inner: Arc<AtomicPtr<CCasUnion<T>>>
}

impl<T> CCasPtr<T> {
    pub fn c_cas(&self, expect: *mut CCasUnion<T>, new: *mut CCasUnion<T>, cond: Arc<AtomicPtr<Status>>) {
        let mut desc = CCasUnion::CCasDesc(CCasDesc::<T> {
            inner: self.inner.clone(),
            expect,
            new,
            cond: cond.clone()
        });

        let expect_ptr = desc.borrow_mut_c_cas_desc().expect;
        let desc_ptr = &mut desc as *mut CCasUnion<T>;

        loop {
            let res = desc.borrow_mut_c_cas_desc().inner.compare_and_swap(expect_ptr, desc_ptr, Ordering::SeqCst); // TODO: set order carefully
            if std::ptr::eq(res, desc.borrow_mut_c_cas_desc().expect) {
                desc.borrow_mut_c_cas_desc().help(desc_ptr);
            } else {
                match unsafe { &*res } {
                    CCasUnion::CCasDesc(c_cas_desc) => c_cas_desc.help(desc_ptr),
                    _ => return // TODO: mark failed
                }
            }
        }
    }

    pub fn load(&self) -> *mut T {
        loop {
            unsafe {
                let v = &mut *self.inner.load(Ordering::SeqCst) as *mut CCasUnion<T>; // TODO: set order carefully
                match &mut *v {
                    CCasUnion::CCasDesc(c_cas_desc) => c_cas_desc.help(v),
                    CCasUnion::Value(val) => return val as *mut T
                }
            }
        }
    }
}