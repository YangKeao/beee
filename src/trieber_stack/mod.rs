use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering;
use std::cmp::PartialEq;
use std::ptr::null_mut;

struct Node<T: PartialEq> {
    val: T,
    next: AtomicPtr<Option<Node<T>>>
}

impl<T: PartialEq> PartialEq for Node<T> {
    fn eq(&self, other: &Node<T>) -> bool {
        self.val == other.val
    }
}

pub struct Stack<T: PartialEq> {
    top: AtomicPtr<Option<Node<T>>>
}

impl<T: PartialEq> Stack<T> {
    pub fn push(&self, val: T) {
        let node = Box::new(Some(Node {
            val,
            next: AtomicPtr::new(null_mut())
        }));
        let node_ptr = Box::leak(node);

        loop {
            let top = self.top.load(Ordering::Relaxed);
            match node_ptr {
                Some(node) => {
                    node.next = AtomicPtr::new(top);
                }
                None => {
                    unreachable!()
                }
            }

            if let Ok(_) = self.top.compare_exchange(top, node_ptr as *mut Option<Node<T>>, Ordering::SeqCst, Ordering::Relaxed) {
                break;
            }
        }
    }

    pub fn pop(&self) {
        loop {
        }
    }
}