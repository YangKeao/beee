use crate::cas_utils::m_cas::{AtomicMCasPtr, MCasPtr, SingleCas, MCas};
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

    pub fn pop(&self) -> Option<T> {
        loop {
            let top = self.head.read();
            match top {
                Some(top) => {
                    let origin_head = self.head.get_m_cas_ptr(Ordering::Relaxed);
                    let next = top.next.get_m_cas_ptr(Ordering::Relaxed);
                    let cas = SingleCas::new(
                        &self.head,
                        origin_head,
                        next
                    );

                    if vec![cas].m_cas() {
                        let retired_head = unsafe { &mut *(*origin_head).read_mut()};
                        return Some(retired_head.take().unwrap().val);
                    }
                }
                None => {return None;}
            }
        }
    }
}
