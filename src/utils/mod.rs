use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::sync::Arc;

#[cfg(target_has_atomic = "ptr")]
#[cfg_attr(target_pointer_width = "16", repr(C, align(2)))]
#[cfg_attr(target_pointer_width = "32", repr(C, align(4)))]
#[cfg_attr(target_pointer_width = "64", repr(C, align(8)))]
struct PubAtomicPtr<T> {
    pub p: UnsafeCell<*mut T>,
}

pub trait AtomicPtrAddOn<T> {
    unsafe fn get_addr(&self) -> u64;
}
impl<T> AtomicPtrAddOn<T> for AtomicPtr<T> {
    unsafe fn get_addr(&self) -> u64 {
        let unsafe_cell: &PubAtomicPtr<T> =
            std::mem::transmute::<&AtomicPtr<T>, &PubAtomicPtr<T>>(self);
        std::mem::transmute::<*mut T, u64>(*unsafe_cell.p.get())
    }
}

pub struct AtomicNumLikes {
    inner: Arc<AtomicUsize>,
}

pub trait AtomicNumLikesMethods<T: From<usize> + Into<usize> + Copy> {
    fn new(v: T) -> AtomicNumLikes;
    fn get(&self, order: Ordering) -> T;
    fn compare_and_swap(&self, current: T, new: T, order: Ordering) -> T;
}

impl<T: From<usize> + Into<usize> + Copy> AtomicNumLikesMethods<T> for AtomicNumLikes {
    fn new(v: T) -> AtomicNumLikes {
        AtomicNumLikes {
            inner: Arc::new(AtomicUsize::new(v.into())),
        }
    }

    fn get(&self, order: Ordering) -> T {
        T::from(self.inner.load(order).clone())
    }

    fn compare_and_swap(&self, current: T, new: T, order: Ordering) -> T {
        T::from(
            self.inner
                .compare_and_swap(current.into(), new.into(), order),
        )
    }
}
