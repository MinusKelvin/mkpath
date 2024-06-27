use std::ops::{IndexMut, Range};

use mkpath_core::traits::{Expander, NodePool, Successor};
use mkpath_core::{NodeAllocator, NodeBuilder, NodeMemberPointer};

pub trait ExplicitStateSpace {
    type State: Copy + 'static;
    type Auxiliary<T>: IndexMut<Self::State, Output = T>;
    type NodePool: NodePool<State = Self::State>;
    type Expander<'a>: Expander<'a>
    where
        Self: 'a;

    fn new_auxiliary<T>(&self, init: impl FnMut(Self::State) -> T) -> Self::Auxiliary<T>;

    fn add_state_field(&self, builder: &mut NodeBuilder) -> NodeMemberPointer<Self::State>;

    fn new_node_pool(
        &self,
        alloc: NodeAllocator,
        state: NodeMemberPointer<Self::State>,
    ) -> Self::NodePool;

    fn new_expander<'a>(
        &'a self,
        node_pool: &'a Self::NodePool,
        state: NodeMemberPointer<Self::State>,
    ) -> Self::Expander<'a>;

    fn list_valid_states(&self) -> Vec<Self::State>;
}

pub struct Mapper<S: ExplicitStateSpace> {
    from_id: Vec<S::State>,
    to_id: S::Auxiliary<usize>,
    component_ends: Vec<usize>,
}

impl<S: ExplicitStateSpace> Mapper<S> {
    pub fn dfs_preorder(domain: &S) -> Self
    where
        for<'a> <S::Expander<'a> as Expander<'a>>::Edge: Successor<'a>,
    {
        let states = domain.list_valid_states();
        let mut from_id = Vec::with_capacity(states.len());
        let mut to_id = domain.new_auxiliary(|_| usize::MAX);
        let mut component_ends = vec![];

        let mut builder = NodeBuilder::new();
        let state = domain.add_state_field(&mut builder);
        let mut node_pool = domain.new_node_pool(builder.build(), state);

        for s in states {
            if to_id[s] != usize::MAX {
                continue;
            }

            node_pool.reset();
            let mut expander = domain.new_expander(&node_pool, state);

            let start = node_pool.generate(s);

            to_id[s] = from_id.len();
            from_id.push(s);
            let mut edges = vec![];
            expander.expand(start, &mut edges);
            let mut stack = vec![edges.into_iter()];

            while let Some(iter) = stack.last_mut() {
                match iter.next() {
                    Some(edge) => {
                        let node = edge.successor();
                        let s = node.get(state);
                        if to_id[s] != usize::MAX {
                            continue;
                        }

                        to_id[s] = from_id.len();
                        from_id.push(s);
                        edges = vec![];
                        expander.expand(node, &mut edges);
                        stack.push(edges.into_iter());
                    }
                    None => {
                        stack.pop();
                    }
                }
            }

            // At the end of the DFS, we have located every state in the connected component.
            component_ends.push(from_id.len());
        }

        Mapper {
            from_id,
            to_id,
            component_ends,
        }
    }

    pub fn states(&self) -> usize {
        self.from_id.len()
    }

    pub fn components(&self) -> usize {
        self.component_ends.len()
    }

    pub fn to_id(&self, s: S::State) -> usize {
        self.to_id[s]
    }

    pub fn to_state(&self, id: usize) -> S::State {
        self.from_id[id]
    }

    pub fn component_id(&self, s: S::State) -> usize {
        let id = self.to_id(s);
        self.component_ends.partition_point(|&end| end <= id)
    }

    pub fn component_id_range(&self, component_id: usize) -> Range<usize> {
        let end = self.component_ends[component_id];
        let start = match component_id {
            0 => 0,
            _ => self.component_ends[component_id - 1],
        };
        start..end
    }

    pub fn same_component(&self, s1: S::State, s2: S::State) -> bool {
        self.component_id_range(self.component_id(s1))
            .contains(&self.to_id(s2))
    }
}
