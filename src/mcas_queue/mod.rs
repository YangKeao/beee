use crate::cas_utils::c_cas::CCasUnion;
use crate::cas_utils::m_cas::{AtomicMCasPtr, MCasPtr, MCasRead, MCasUnion, SingleCas};
use std::sync::atomic::Ordering;

pub struct Node<T> {
    pub val: T,
    pub(crate) next: AtomicMCasPtr<Option<Node<T>>>,
}

pub struct Queue<T> {
    pub head: AtomicMCasPtr<Option<Node<T>>>,
    pub tail: AtomicMCasPtr<Option<Node<T>>>,
}

impl<T> Queue<T> {
    pub fn new() -> Queue<T> {
        let mut none = MCasPtr::new(None);
        return Queue::<T> {
            head: AtomicMCasPtr::new(&mut none),
            tail: AtomicMCasPtr::new(&mut none),
        };
    }
}
