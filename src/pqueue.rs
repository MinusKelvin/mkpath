use std::cmp::Reverse;

use crate::node::*;

pub struct PriorityQueueFactory {
    index: NodeMemberPointer<usize>,
}

pub struct PriorityQueue<'a, C> {
    cmp: C,
    index: NodeMemberPointer<usize>,
    heap: Vec<NodeRef<'a>>,
}

pub unsafe trait FieldComparator {
    unsafe fn le(&self, lhs: NodeRef, rhs: NodeRef) -> bool;

    fn same_layout(&self, index: NodeMemberPointer<usize>) -> bool;
}

impl PriorityQueueFactory {
    pub fn new(builder: &mut NodeBuilder) -> Self {
        PriorityQueueFactory {
            index: builder.add_field(usize::MAX),
        }
    }

    pub fn new_queue<C: FieldComparator>(&mut self, cmp: C) -> PriorityQueue<C> {
        assert!(cmp.same_layout(self.index));
        PriorityQueue {
            cmp,
            index: self.index,
            heap: vec![],
        }
    }
}

impl<'a, C: FieldComparator> PriorityQueue<'a, C> {
    pub fn push(&mut self, node: NodeRef<'a>) {
        let index = node.get(self.index);
        if index >= self.heap.len() || !self.heap[index].same_ptr(node) {
            self.heap.push(node);
            unsafe {
                self.sift_up(node, self.heap.len() - 1);
            }
        } else {
            unsafe {
                self.sift_up(node, index);
            }
        }
    }

    pub fn pop(&mut self) -> Option<NodeRef<'a>> {
        if self.heap.is_empty() {
            return None;
        }
        let ret = self.heap.swap_remove(0);
        if let Some(&node) = self.heap.first() {
            unsafe {
                self.sift_down(node, 0);
            }
        }
        Some(ret)
    }

    unsafe fn sift_up(&mut self, node: NodeRef<'a>, mut index: usize) {
        unsafe {
            while index > 0 {
                let parent_index = (index - 1) / 2;
                let parent = *self.heap.get_unchecked(parent_index);
                if self.cmp.le(parent, node) {
                    break;
                }
                *self.heap.get_unchecked_mut(index) = parent;
                parent.set_unchecked(self.index, index);
                index = parent_index;
            }
            *self.heap.get_unchecked_mut(index) = node;
            node.set_unchecked(self.index, index);
        }
    }

    unsafe fn sift_down(&mut self, node: NodeRef<'a>, mut index: usize) {
        unsafe {
            loop {
                let child_1_index = index * 2 + 1;
                if child_1_index >= self.heap.len() {
                    break;
                }
                let child_1 = self.heap[child_1_index];

                let child_index;
                let child;

                let child_2_index = child_1_index + 1;
                if child_2_index < self.heap.len() {
                    let child_2 = self.heap[child_2_index];

                    if self.cmp.le(child_1, child_2) {
                        child_index = child_1_index;
                        child = child_1;
                    } else {
                        child_index = child_2_index;
                        child = child_2;
                    }
                } else {
                    child_index = child_1_index;
                    child = child_1;
                }

                if self.cmp.le(node, child) {
                    break;
                }

                *self.heap.get_unchecked_mut(index) = child;
                child.set_unchecked(self.index, index);
                index = child_index;
            }

            *self.heap.get_unchecked_mut(index) = node;
            node.set_unchecked(self.index, index);
        }
    }
}

unsafe impl<T: PartialOrd + Copy + 'static> FieldComparator for NodeMemberPointer<T> {
    unsafe fn le(&self, lhs: NodeRef, rhs: NodeRef) -> bool {
        unsafe { lhs.get_unchecked(*self) <= rhs.get_unchecked(*self) }
    }

    fn same_layout(&self, index: NodeMemberPointer<usize>) -> bool {
        self.same_layout(index)
    }
}

unsafe impl<T: PartialOrd + Copy + 'static> FieldComparator for Reverse<NodeMemberPointer<T>> {
    unsafe fn le(&self, lhs: NodeRef, rhs: NodeRef) -> bool {
        unsafe { lhs.get_unchecked(self.0) >= rhs.get_unchecked(self.0) }
    }

    fn same_layout(&self, index: NodeMemberPointer<usize>) -> bool {
        self.0.same_layout(index)
    }
}

unsafe impl<A: FieldComparator, B: FieldComparator> FieldComparator for (A, B) {
    unsafe fn le(&self, lhs: NodeRef, rhs: NodeRef) -> bool {
        if self.0.le(lhs, rhs) {
            if self.0.le(rhs, lhs) {
                self.1.le(lhs, rhs)
            } else {
                true
            }
        } else {
            false
        }
    }

    fn same_layout(&self, index: NodeMemberPointer<usize>) -> bool {
        self.0.same_layout(index) && self.1.same_layout(index)
    }
}
