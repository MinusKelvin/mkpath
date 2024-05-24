use crate::node::{NodeAllocator, NodeMemberPointer, NodeRef};
use crate::traits::NodePool;

pub struct NullPool<S: Copy> {
    state_field: NodeMemberPointer<S>,
    allocator: NodeAllocator,
}

impl<S: Copy + 'static> NullPool<S> {
    #[track_caller]
    pub fn new(allocator: NodeAllocator, state_field: NodeMemberPointer<S>) -> Self {
        assert!(allocator.layout_id() == state_field.layout_id(), "mismatched layouts");
        NullPool {
            state_field,
            allocator,
        }
    }

    pub fn get(&self, _state: &S) -> Option<NodeRef> {
        None
    }
}

impl<S: Copy + 'static> NodePool for NullPool<S> {
    type State = S;

    fn reset(&mut self) {
        self.allocator.reset();
    }

    fn generate(&self, state: S) -> NodeRef {
        let node = self.allocator.new_node();
        node.set(self.state_field, state);
        node
    }
}
