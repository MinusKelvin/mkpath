use crate::node::{NodeAllocator, NodeMemberPointer, NodeRef};

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

    pub fn reset(&mut self) {
        self.allocator.reset();
    }

    pub fn generate(&self, state: S) -> NodeRef {
        let node = self.allocator.generate_node();
        node.set(self.state_field, state);
        node
    }
}
