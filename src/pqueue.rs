use crate::node::*;

pub struct PriorityQueueFactory {
    index: NodeMemberPointer<usize>,
}

pub struct PriorityQueue<'a, K> {
    key: NodeMemberPointer<K>,
    index: NodeMemberPointer<usize>,
    heap: Vec<NodeRef<'a>>,
}

impl PriorityQueueFactory {
    pub fn new(builder: &mut NodeBuilder) -> Self {
        PriorityQueueFactory {
            index: builder.add_field(usize::MAX),
        }
    }

    pub fn new_queue<K: Copy>(&mut self, key: NodeMemberPointer<K>) -> PriorityQueue<K> {
        assert!(self.index.same_layout(key));
        PriorityQueue {
            key,
            index: self.index,
            heap: vec![],
        }
    }
}

impl<'a, K: PartialOrd + Copy + 'static> PriorityQueue<'a, K> {
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
            let key = node.get_unchecked(self.key);
            while index > 0 {
                let parent_index = (index - 1) / 2;
                let parent = *self.heap.get_unchecked(parent_index);
                if key >= parent.get_unchecked(self.key) {
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
            let key = node.get_unchecked(self.key);
            loop {
                let child_1_index = index * 2 + 1;
                if child_1_index >= self.heap.len() {
                    break;
                }
                let child_1 = self.heap[child_1_index];
                let child_1_key = child_1.get_unchecked(self.key);

                let child_index;
                let child;
                let child_key;

                let child_2_index = child_1_index + 1;
                if child_2_index < self.heap.len() {
                    let child_2 = self.heap[child_2_index];
                    let child_2_key = child_2.get_unchecked(self.key);

                    if child_1_key <= child_2_key {
                        child_index = child_1_index;
                        child = child_1;
                        child_key = child_1_key;
                    } else {
                        child_index = child_2_index;
                        child = child_2;
                        child_key = child_2_key;
                    }
                } else {
                    child_index = child_1_index;
                    child = child_1;
                    child_key = child_1_key;
                }

                if child_key >= key {
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
