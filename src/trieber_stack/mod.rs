use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering;
use std::cmp::PartialEq;

struct Node<T: PartialEq> {
    val: T,
    next: Option<AtomicPtr<Node<T>>>
}

impl<T: PartialEq> PartialEq for Node<T> {
    fn eq(&self, other: &Node<T>) -> bool {
        self.val == other.val
    }
}

pub struct Stack<T: PartialEq> {
    top: AtomicPtr<Node<T>>
}

impl<T: PartialEq> Stack<T> {
    pub fn push(&self, val: T) {
        let node = Box::new(Node {
            val,
            next: None
        });
        let node_ptr = Box::leak(node);

        loop {
            let top = self.top.load(Ordering::Relaxed);
            node_ptr.next = Some(AtomicPtr::new(top));

            if let Ok(_) = self.top.compare_exchange(top, node_ptr as *mut Node<T>, Ordering::SeqCst, Ordering::Relaxed) {
                break;
            }
        }
    }
}