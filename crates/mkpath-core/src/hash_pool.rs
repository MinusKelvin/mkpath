use std::cell::RefCell;
use std::hash::Hash;
use std::ptr::NonNull;

use ahash::AHashMap;

use crate::node::{Node, NodeAllocator, NodeMemberPointer, NodeRef};

pub struct HashPool<S: Copy> {
    state_field: NodeMemberPointer<S>,
    allocator: NodeAllocator,
    // We use RefCell instead of UnsafeCell since the Hash implementation for S could
    // theoretically reentrantly call HashPool::generate, which would cause UB.
    map: RefCell<AHashMap<S, NonNull<Node>>>,
}

impl<S: Copy + Hash + Eq + 'static> HashPool<S> {
    #[track_caller]
    pub fn new(allocator: NodeAllocator, state_field: NodeMemberPointer<S>) -> Self {
        assert!(
            allocator.layout_id() == state_field.layout_id(),
            "mismatched layouts"
        );
        HashPool {
            state_field,
            allocator,
            map: RefCell::new(AHashMap::new()),
        }
    }

    pub fn state_member(&self) -> NodeMemberPointer<S> {
        self.state_field
    }

    pub fn reset(&mut self) {
        self.map.get_mut().clear();
        self.allocator.reset();
    }

    pub fn generate(&self, state: S) -> NodeRef {
        unsafe {
            NodeRef::from_raw(*self.map.borrow_mut().entry(state).or_insert_with(|| {
                let node = self.allocator.generate_node();
                node.set(self.state_field, state);
                node.raw()
            }))
        }
    }

    pub fn get(&self, state: &S) -> Option<NodeRef> {
        self.map
            .borrow()
            .get(state)
            .map(|&ptr| unsafe { NodeRef::from_raw(ptr) })
    }
}
