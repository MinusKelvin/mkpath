use std::collections::VecDeque;

use mkpath_core::traits::OpenList;
use mkpath_core::{NodeBuilder, NodeMemberPointer, NodeRef};

pub struct BucketQueueFactory {
    bucket_pos: NodeMemberPointer<(u32, u32)>,
}

impl BucketQueueFactory {
    pub fn new(builder: &mut NodeBuilder) -> Self {
        BucketQueueFactory {
            bucket_pos: builder.add_field((u32::MAX, u32::MAX)),
        }
    }

    pub fn new_queue<'a>(&self, g: NodeMemberPointer<f64>, bucket_width: f64) -> BucketQueue<'a> {
        assert!(g.layout_id() == self.bucket_pos.layout_id());
        BucketQueue {
            bucket_number: 0,
            bucket_width,
            g,
            bucket_pos: self.bucket_pos,
            queue: VecDeque::new(),
        }
    }
}

pub struct BucketQueue<'a> {
    bucket_number: u32,
    bucket_width: f64,
    g: NodeMemberPointer<f64>,
    bucket_pos: NodeMemberPointer<(u32, u32)>,
    queue: VecDeque<Vec<NodeRef<'a>>>,
}

impl<'a> OpenList<'a> for BucketQueue<'a> {
    fn next(&mut self) -> Option<NodeRef<'a>> {
        while let Some(front) = self.queue.front_mut() {
            if let Some(node) = front.pop() {
                return Some(node);
            }
            let old = self.queue.pop_front().unwrap();
            if self.queue.back().is_some_and(|vec| !vec.is_empty()) {
                self.queue.push_back(old);
            }
            self.bucket_number += 1;
        }
        None
    }

    #[inline(always)]
    fn relaxed(&mut self, node: NodeRef<'a>) {
        let (bucket, index) = node.get(self.bucket_pos);
        let new_bucket = (node.get(self.g) / self.bucket_width) as u32;
        if bucket == new_bucket {
            return;
        }

        if bucket != u32::MAX {
            let old_bucket = &mut self.queue[(bucket - self.bucket_number) as usize];
            debug_assert!(old_bucket[index as usize].ptr_eq(node));
            if let Some(swapped_in) = old_bucket.pop() {
                if !swapped_in.ptr_eq(node) {
                    old_bucket[index as usize] = swapped_in;
                    swapped_in.set(self.bucket_pos, (bucket, index));
                }
            }
        }

        let new_index = (new_bucket - self.bucket_number) as usize;
        if new_index >= self.queue.len() {
            self.queue.resize(new_index + 1, vec![]);
        }
        let bucket = &mut self.queue[new_index];
        node.set(self.bucket_pos, (new_bucket, bucket.len() as u32));
        bucket.push(node);
    }
}