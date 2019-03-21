use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering;
use std::cmp::PartialEq;
use std::ptr::null_mut;

struct Node<T: PartialEq + Copy> {
    pub val: T,
    pub next: AtomicPtr<Option<Node<T>>>
}

impl<T: PartialEq + Copy> PartialEq for Node<T> {
    fn eq(&self, other: &Node<T>) -> bool {
        self.val == other.val
    }
}

pub struct Stack<T: PartialEq + Copy> {
    top: AtomicPtr<Option<Node<T>>>
}

impl<T: PartialEq + Copy> Stack<T> {
    pub fn new() -> Stack<T> {
        let none = Box::new(None);
        let none_ptr = Box::leak(none);
        return Stack {
            top: AtomicPtr::new(none_ptr)
        }
    }
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

    pub fn pop(&self) -> Option<T> {
        loop {
            let top = self.top.load(Ordering::Relaxed);
            match unsafe{&mut *top} {
                Some(n) => {
                    if let Ok(_) = self.top.compare_exchange(top, n.next.load(Ordering::Relaxed), Ordering::SeqCst, Ordering::Relaxed) {
                        break Some(n.val);
                    }
                }
                None => {
                    break None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::sync::Arc;

    #[test]
    fn single_thread_push() {
        let s = Stack::new();
        for i in 0..1<<20 {
            s.push(i);
        }
    }

    #[test]
    fn single_thread_pop() {
        let s = Stack::new();
        for i in 0..1<<20 {
            s.push(i);
        }
        for _ in 0..1<<20 {
            s.pop();
        }
    }

    #[test]
    fn two_thread_push_and_pop() {
        let s = Arc::new(Stack::new());
        let c_s = s.clone();
        let push_thread = thread::spawn(move|| {
            for i in 0..1<<20 {
                s.push(i);
            }
        });
        let pop_thread = thread::spawn(move|| {
            for _ in 0..1<<20 {
                c_s.pop();
            }
        });
        push_thread.join().unwrap();
        pop_thread.join().unwrap();
    }

    #[test]
    fn multi_thread_push_and_pop() {
        let s = Arc::new(Stack::new());
        let push_threads = (0..10).map(|_| {
            let c_s = s.clone();
            thread::spawn(move|| {
                for _ in 0..1<<20 {
                    c_s.push(0);
                }
            })
        });
        for push_thread in push_threads {
            push_thread.join().unwrap();
        }
        let pop_threads = (0..10).map(|_| {
            let c_s = s.clone();
            thread::spawn(move|| {
                for _ in 0..1<<20 {
                    let res = c_s.pop();
                    assert_eq!(res, Some(0));
                }
            })
        });
        for pop_thread in pop_threads {
            pop_thread.join().unwrap();
        }
    }
}