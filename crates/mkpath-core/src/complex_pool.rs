use std::cell::{Ref, RefCell};
use std::hash::Hash;
use std::ptr::NonNull;

use ahash::AHashMap;

use crate::{Node, NodeAllocator, NodeBuilder, NodeMemberPointer, NodeRef};

pub struct ComplexStatePool<S> {
    allocator: NodeAllocator,
    state_field: NodeMemberPointer<usize>,
    data: RefCell<Data<S>>,
}

struct Data<S> {
    map: AHashMap<S, NonNull<Node>>,
    states: Vec<S>,
}

impl<S: Hash + Eq + Clone> ComplexStatePool<S> {
    pub fn new(mut builder: NodeBuilder) -> Self {
        let state_field = builder.add_field(usize::MAX);
        ComplexStatePool {
            allocator: builder.build(),
            state_field,
            data: RefCell::new(Data {
                map: AHashMap::new(),
                states: vec![],
            }),
        }
    }

    pub fn get_state(&self, node: NodeRef) -> Ref<S> {
        let index = node.get(self.state_field);
        Ref::map(self.data.borrow(), |data| &data.states[index])
    }

    pub fn reset(&mut self) {
        let data = self.data.get_mut();
        data.map.clear();
        data.states.clear();
        self.allocator.reset();
    }

    pub fn generate(&self, state: S) -> NodeRef {
        unsafe {
            let mut data = self.data.borrow_mut();
            let Data { map, states } = &mut *data;
            NodeRef::from_raw(*map.entry(state).or_insert_with_key(|state| {
                let index = states.len();
                states.push(state.clone());
                let node = self.allocator.generate_node();
                node.set(self.state_field, index);
                node.raw()
            }))
        }
    }

    pub fn get(&self, state: &S) -> Option<NodeRef> {
        self.data
            .borrow()
            .map
            .get(state)
            .map(|&ptr| unsafe { NodeRef::from_raw(ptr) })
    }
}
