use std::cell::Cell;
use std::ptr::NonNull;

use mkpath_core::{
    Node, NodeAllocator, NodeBuilder, NodeMemberPointer, NodeRef, PriorityQueueFactory,
};

pub trait Graph {
    fn num_states(&self) -> usize;

    type OutgoingEdgesIter<'a>: Iterator<Item = Edge> + 'a
    where
        Self: 'a;
    fn outgoing_edges(&self, id: usize) -> Self::OutgoingEdgesIter<'_>;
}

pub struct Edge {
    pub to_vertex: usize,
    pub edge_id: usize,
    pub cost: f64,
}

pub struct Cpd {
    remapped: Vec<usize>,
    rows: Vec<Vec<(usize, usize)>>,
}

impl Cpd {
    pub fn compute(graph: &impl Graph) -> Self {
        let remapped = dfs_preorder(graph);

        let mut searcher = DijkstraSearcher::new(graph.num_states());

        let rows = (0..graph.num_states())
            .map(|v| compute_row(&mut searcher, graph, &remapped, v))
            .collect();

        Cpd { remapped, rows }
    }

    pub fn lookup(&self, vertex: usize, target: usize) -> usize {
        let index = self.rows[vertex]
            .binary_search_by_key(&self.remapped[target], |&(i, _)| i)
            .unwrap_or_else(|i| i - 1);
        self.rows[vertex][index].1
    }
}

fn dfs_preorder(graph: &impl Graph) -> Vec<usize> {
    let mut remapped = vec![usize::MAX; graph.num_states()];

    let mut i = 0;
    while i < remapped.len() {
        let root = remapped
            .iter()
            .enumerate()
            .find(|(_, &id)| id == usize::MAX)
            .unwrap()
            .0;
        remapped[root] = i;
        i += 1;
        let mut stack = vec![graph.outgoing_edges(root)];
        while let Some(iter) = stack.last_mut() {
            let Some(edge) = iter.next() else {
                stack.pop();
                continue;
            };
            if remapped[edge.to_vertex] == usize::MAX {
                remapped[edge.to_vertex] = i;
                i += 1;
                stack.push(graph.outgoing_edges(edge.to_vertex));
            }
        }
    }

    remapped
}

fn compute_row(
    searcher: &mut DijkstraSearcher,
    graph: &impl Graph,
    remapped: &[usize],
    vertex: usize,
) -> Vec<(usize, usize)> {
    searcher.search(graph, vertex);
    let mut uncompressed: Vec<_> = (0..remapped.len())
        .map(|i| {
            (
                remapped[i],
                searcher
                    .pool
                    .get(i)
                    .map_or(0, |node| node.get(searcher.first_move)),
            )
        })
        .collect();
    uncompressed.sort_by_key(|&(i, _)| i);
    uncompressed.dedup_by_key(|&mut (_, mv)| mv);
    uncompressed
}

struct DijkstraSearcher {
    pool: ArrayPool,
    pqueue_factory: PriorityQueueFactory,
    first_move: NodeMemberPointer<usize>,
    g: NodeMemberPointer<f64>,
}

impl DijkstraSearcher {
    fn new(num_vertices: usize) -> Self {
        let mut builder = NodeBuilder::new();
        let state = builder.add_field(0);
        let g = builder.add_field(f64::INFINITY);
        let first_move = builder.add_field(0);
        let pqueue_factory = PriorityQueueFactory::new(&mut builder);
        let pool = ArrayPool::new(builder.build(), state, num_vertices);

        DijkstraSearcher {
            pool,
            pqueue_factory,
            first_move,
            g,
        }
    }

    fn search(&mut self, graph: &impl Graph, start: usize) {
        let pool = &mut self.pool;
        let g = self.g;
        let first_move = self.first_move;

        pool.reset();

        let mut pqueue = self.pqueue_factory.new_queue(g);

        let start = pool.generate(start);
        start.set(g, 0.0);
        for edge in graph.outgoing_edges(start.get(pool.state_member())) {
            let new_node = pool.generate(edge.to_vertex);
            new_node.set_parent(Some(start));
            new_node.set(g, edge.cost);
            new_node.set(first_move, edge.edge_id);
        }

        while let Some(node) = pqueue.pop() {
            for edge in graph.outgoing_edges(node.get(pool.state_member())) {
                let new_node = pool.generate(edge.to_vertex);
                let new_g = node.get(g) + edge.cost;
                if new_g < new_node.get(g) {
                    new_node.set_parent(Some(node));
                    new_node.set(g, new_g);
                    new_node.set(first_move, node.get(first_move));
                }
            }
        }
    }
}

struct ArrayPool {
    state_map: Box<[Cell<(u64, *mut Node)>]>,
    search_number: u64,
    state_field: NodeMemberPointer<usize>,
    allocator: NodeAllocator,
}

impl ArrayPool {
    #[track_caller]
    pub fn new(
        allocator: NodeAllocator,
        state_field: NodeMemberPointer<usize>,
        vertices: usize,
    ) -> Self {
        assert!(
            allocator.layout_id() == state_field.layout_id(),
            "mismatched layouts"
        );

        ArrayPool {
            search_number: 1,
            state_map: vec![Cell::new((0, std::ptr::null_mut())); vertices].into_boxed_slice(),
            state_field,
            allocator,
        }
    }

    #[inline(always)]
    pub fn vertices(&self) -> usize {
        self.state_map.len()
    }

    #[inline(always)]
    pub fn state_member(&self) -> NodeMemberPointer<usize> {
        self.state_field
    }

    pub fn reset(&mut self) {
        self.search_number = self.search_number.checked_add(1).unwrap_or_else(|| {
            self.state_map.fill(Cell::new((0, std::ptr::null_mut())));
            1
        });
        self.allocator.reset();
    }

    #[track_caller]
    #[inline(always)]
    pub fn generate(&self, state: usize) -> NodeRef {
        let _ = self.state_map[state];
        unsafe { self.generate_unchecked(state) }
    }

    #[track_caller]
    #[inline(always)]
    pub fn get(&self, state: usize) -> Option<NodeRef> {
        let _ = self.state_map[state];
        unsafe { self.get_unchecked(state) }
    }

    /// Retrieves the node for the specified state or generates one if it does not exist, without
    /// performing bounds checks.
    ///
    /// # Safety
    /// The coordinates must be in-bounds of the grid. Specifically:
    /// - `x` is in `0..self.width()`
    /// - `y` is in `0..self.height()`
    #[inline(always)]
    #[cfg_attr(debug_assertions, track_caller)]
    pub unsafe fn generate_unchecked(&self, state: usize) -> NodeRef {
        let slot = unsafe { self.state_map.get_unchecked(state) };
        let (num, ptr) = slot.get();
        if num == self.search_number {
            debug_assert!(!ptr.is_null());
            unsafe { NodeRef::from_raw(NonNull::new_unchecked(ptr)) }
        } else {
            let ptr = self.allocator.new_node();
            unsafe {
                ptr.set_unchecked(self.state_field, state);
            }
            slot.set((self.search_number, ptr.into_raw().as_ptr()));
            ptr
        }
    }

    /// Retrieves the node for the specified state if it exists, without performing bounds checks.
    ///
    /// # Safety
    /// The coordinates must be in-bounds of the grid. Specifically:
    /// - `x` is in `0..self.width()`
    /// - `y` is in `0..self.height()`
    #[inline(always)]
    #[cfg_attr(debug_assertions, track_caller)]
    pub unsafe fn get_unchecked(&self, state: usize) -> Option<NodeRef> {
        let slot = unsafe { self.state_map.get_unchecked(state) };
        let (num, ptr) = slot.get();
        if num == self.search_number {
            unsafe { Some(NodeRef::from_raw(NonNull::new_unchecked(ptr))) }
        } else {
            None
        }
    }
}
