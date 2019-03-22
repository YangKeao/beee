use crate::utils::AtomicPtrAddOn;
use std::cell::UnsafeCell;
use std::cmp::Ordering;
use std::sync::atomic::AtomicPtr;

pub(crate) struct Cas<T> {
    ptr: UnsafeCell<AtomicPtr<T>>,
    expects: AtomicPtr<T>,
    new: AtomicPtr<T>,
}

impl<T> Ord for Cas<T> {
    fn cmp(&self, other: &Cas<T>) -> Ordering {
        unsafe { self.ptr.get_addr().cmp(&other.ptr.get_addr()) }
    }
}

impl<T> PartialOrd for Cas<T> {
    fn partial_cmp(&self, other: &Cas<T>) -> Option<Ordering> {
        unsafe { self.ptr.get_addr().partial_cmp(&other.ptr.get_addr()) }
    }
}

impl<T> PartialEq for Cas<T> {
    fn eq(&self, other: &Cas<T>) -> bool {
        unsafe { self.ptr.get_addr() == other.ptr.get_addr() }
    }
}

impl<T> Eq for Cas<T> {}

enum Status {
    Undecided,
    Failed,
    Successful,
}

struct MCasDesc<T> {
    cases: Vec<Cas<T>>,
    status: Box<Status>,
}

impl<T> MCasDesc<T> {
    fn help(&self) {
        let success = false;
        for item in self.cases {
            loop {

            }
        }
    }
}

enum Ptr<T> {
    MCasDesc(MCasDesc<T>),
    Ptr(AtomicPtr<T>),
}

pub(crate) fn m_cas<T>(cases: Vec<Cas<T>>) -> bool {
    let mut m_cas_desc = MCasDesc {
        cases,
        status: Box::new(Status::Undecided),
    };
    m_cas_desc.cases.sort();
    return m_cas_desc.help();
}

struct CCasDesc<T> {
    ptr: AtomicPtr<T>,
    expects: AtomicPtr<T>,
    new: AtomicPtr<T>,
    cond: Box<Status>,
}

pub(crate) fn c_cas<T>(ptr: AtomicPtr<T>, expects: AtomicPtr<T>, new: AtomicPtr<T>, cond: &Status) -> bool {
    
}