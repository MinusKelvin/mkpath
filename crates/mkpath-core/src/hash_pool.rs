use std::cell::RefCell;
use std::hash::Hash;
use std::ptr::NonNull;

use ahash::AHashMap;

use crate::node::{Node, NodeAllocator, NodeMemberPointer, NodeRef};
use crate::traits::NodePool;

pub struct HashPool<S> {
    state_field: NodeMemberPointer<S>,
    allocator: NodeAllocator,
    // We use RefCell instead of UnsafeCell since the Hash implementation for S could
    // theoretically re-entrantly call HashPool::generate, which would cause UB.
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

    pub fn get(&self, state: &S) -> Option<NodeRef> {
        self.map
            .borrow()
            .get(state)
            .map(|&ptr| unsafe { NodeRef::from_raw(ptr) })
    }
}

impl<S: Copy + Hash + Eq + 'static> NodePool for HashPool<S> {
    type State = S;

    fn reset(&mut self) {
        self.map.get_mut().clear();
        self.allocator.reset();
    }

    fn generate(&self, state: Self::State) -> NodeRef {
        unsafe {
            NodeRef::from_raw(*self.map.borrow_mut().entry(state).or_insert_with(|| {
                let node = self.allocator.new_node();
                node.set(self.state_field, state);
                node.into_raw()
            }))
        }
    }
}
