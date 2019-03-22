use std::cell::UnsafeCell;
use std::sync::atomic::AtomicPtr;

pub trait AtomicPtrAddOn<T> {
    unsafe fn get_addr(&self) -> u64;
}
impl<T> AtomicPtrAddOn<T> for UnsafeCell<AtomicPtr<T>> {
    unsafe fn get_addr(&self) -> u64 {
        let mut_ptr = *((*self.get()).get_mut() as *mut *mut T);
        std::mem::transmute::<*mut T, u64>(mut_ptr)
    }
}
