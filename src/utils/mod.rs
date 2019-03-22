use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::sync::Arc;

pub trait AtomicPtrAddOn<T> {
    unsafe fn get_addr(&self) -> u64;
}
impl<T> AtomicPtrAddOn<T> for UnsafeCell<AtomicPtr<T>> {
    unsafe fn get_addr(&self) -> u64 {
        let mut_ptr = *((*self.get()).get_mut() as *mut *mut T);
        std::mem::transmute::<*mut T, u64>(mut_ptr)
    }
}

pub struct AtomicNumLikes {
    inner: Arc<AtomicUsize>,
}

pub trait AtomicNumLikesMethods<T: From<usize> + Into<usize> + Copy> {
    fn new(v: T) -> AtomicNumLikes;
    fn get(&mut self, order: Ordering) -> T;
    fn compare_and_swap(&self, current: T, new: T, order: Ordering) -> T;
}

impl<T: From<usize> + Into<usize> + Copy> AtomicNumLikesMethods<T> for AtomicNumLikes {
    fn new(v: T) -> AtomicNumLikes {
        AtomicNumLikes {
            inner: Arc::new(AtomicUsize::new(v.into())),
        }
    }

    fn get(&mut self, order: Ordering) -> T {
        T::from(self.inner.load(order).clone())
    }

    fn compare_and_swap(&self, current: T, new: T, order: Ordering) -> T {
        T::from(self.inner.compare_and_swap(current.into(), new.into(), order))
    }
}
