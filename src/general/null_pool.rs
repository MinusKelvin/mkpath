use crate::node::{NodeAllocator, NodeMemberPointer, NodeRef};

pub struct NullPool<S: Copy> {
    state_field: NodeMemberPointer<S>,
    allocator: NodeAllocator,
}

impl<S: Copy + 'static> NullPool<S> {
    #[track_caller]
    pub fn new(allocator: NodeAllocator, state_field: NodeMemberPointer<S>) -> Self {
        assert!(allocator.same_layout(state_field), "mismatched layouts");
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
