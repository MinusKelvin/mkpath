use std::cell::{Ref, RefCell};
use std::hash::Hash;
use std::ptr::NonNull;

use ahash::AHashMap;

use crate::{Node, NodeAllocator, NodeBuilder, NodeMemberPointer, NodeRef};

pub struct ComplexStatePool<S> {
    allocator: NodeAllocator,
    state_field: NodeMemberPointer<usize>,
    data: RefCell<(AHashMap<S, NonNull<Node>>, Vec<S>)>,
}

impl<S: Hash + Eq + Clone> ComplexStatePool<S> {
    pub fn new(mut builder: NodeBuilder) -> Self {
        let state_field = builder.add_field(usize::MAX);
        ComplexStatePool {
            allocator: builder.build(),
            state_field,
            data: RefCell::new((AHashMap::new(), vec![])),
        }
    }

    pub fn get_state(&self, node: NodeRef) -> Ref<S> {
        let index = node.get(self.state_field);
        Ref::map(self.data.borrow(), |(_, states)| &states[index])
    }

    pub fn reset(&mut self) {
        let (map, states) = self.data.get_mut();
        map.clear();
        states.clear();
        self.allocator.reset();
    }

    pub fn generate(&self, state: S) -> NodeRef {
        unsafe {
            let mut borrow = self.data.borrow_mut();
            let (map, states) = &mut *borrow;
            NodeRef::from_raw(*map.entry(state).or_insert_with_key(|state| {
                let index = states.len();
                states.push(state.clone());
                let node = self.allocator.generate_node();
                node.set(self.state_field, index);
                node.raw()
            }))
        }
    }
}
