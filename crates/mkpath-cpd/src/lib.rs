use std::collections::VecDeque;
use std::io::{Read, Write};

use mkpath_core::traits::{Cost, EdgeId, Expander, OpenList, Successor};
use mkpath_core::{NodeBuilder, NodeMemberPointer, NodeRef};

pub trait StateIdMapper {
    type State;

    fn num_ids(&self) -> usize;

    fn state_to_id(&self, state: Self::State) -> usize;

    fn id_to_state(&self, id: usize) -> Self::State;
}

pub fn dfs_traversal<'a, E: Expander<'a, Edge = Edge>, Edge: Successor<'a>>(
    start: NodeRef<'a>,
    mut expander: E,
    mut found: impl FnMut(NodeRef<'a>) -> bool,
) {
    found(start);
    let mut edges = vec![];
    expander.expand(start, &mut edges);
    let mut stack = vec![edges.into_iter()];

    while let Some(edges) = stack.last_mut() {
        if let Some(edge) = edges.next() {
            let node = edge.successor();
            if found(node) {
                let mut new_edges = vec![];
                expander.expand(node, &mut new_edges);
                stack.push(new_edges.into_iter());
            }
        } else {
            stack.pop();
        }
    }
}

#[repr(transparent)]
pub struct CpdRow {
    runs: [CpdEntry],
}

#[derive(Copy, Clone, Debug)]
struct CpdEntry(u32);

impl CpdEntry {
    fn start(self) -> usize {
        (self.0 & (1 << 26) - 1) as usize
    }

    fn edge(self) -> usize {
        (self.0 >> 26) as usize
    }
}

impl CpdRow {
    fn from_raw_box(slice: Box<[CpdEntry]>) -> Box<CpdRow> {
        unsafe {
            // SAFETY: `CpdRow` wraps a `[CpdEntry]` transparently, so this is safe
            std::mem::transmute(slice)
        }
    }

    pub fn compute<'a, M, S, Exp, Edge, Open>(
        mapper: &M,
        searcher: &mut FirstMoveSearcher,
        expander: Exp,
        open: Open,
        start: NodeRef<'a>,
        state: NodeMemberPointer<S>,
    ) -> Box<CpdRow>
    where
        S: Copy + 'static,
        M: StateIdMapper<State = S>,
        Exp: Expander<'a, Edge = Edge>,
        Edge: Successor<'a> + Cost + EdgeId,
        Open: OpenList<'a>,
    {
        assert!(mapper.num_ids() < 1 << 26);
        let mut first_moves = vec![!0; mapper.num_ids()];

        searcher.search(start, expander, open, |node, fm| {
            first_moves[mapper.state_to_id(node.get(state))] = fm
        });

        Self::compress(first_moves)
    }

    pub fn compress(first_move_bits: impl IntoIterator<Item = u64>) -> Box<CpdRow> {
        Self::compress_runs(first_move_bits.into_iter().enumerate())
    }

    pub fn compress_runs(first_move_bits: impl IntoIterator<Item = (usize, u64)>) -> Box<CpdRow> {
        let mut runs = vec![];
        let mut current_id = 0;
        let mut current_moves = !0;
        for (id, moves) in first_move_bits.into_iter().chain(Some((0, 0))) {
            if current_moves & moves == 0 {
                runs.push(CpdEntry(current_id | current_moves.trailing_zeros() << 26));
                current_id = id as u32;
                current_moves = moves;
            } else {
                current_moves &= moves;
            }
        }

        let sorted = runs.clone();
        reorder_eytzinger(&mut sorted.into_iter(), &mut runs, 0);

        Self::from_raw_box(runs.into_boxed_slice())
    }

    pub fn len(&self) -> usize {
        self.runs.len()
    }

    pub fn lookup(&self, id: usize) -> usize {
        let mut i = 0;
        let mut result = 0;
        while i < self.runs.len() {
            if id < self.runs[i].start() {
                i = 2 * i + 1;
            } else {
                result = self.runs[i].edge();
                i = 2 * i + 2;
            }
        }
        result
    }

    pub fn save(&self, to: &mut impl Write) -> std::io::Result<()> {
        to.write_all(&(self.runs.len() as u32).to_le_bytes())?;
        for &run in &self.runs {
            to.write_all(&run.0.to_le_bytes())?;
        }
        Ok(())
    }

    pub fn load(from: &mut impl Read) -> std::io::Result<Box<Self>> {
        let mut bytes = [0; 4];
        from.read_exact(&mut bytes)?;
        let len = u32::from_le_bytes(bytes) as usize;
        let rows = (0..len)
            .map(|_| {
                from.read_exact(&mut bytes)?;
                Ok(CpdEntry(u32::from_le_bytes(bytes)))
            })
            .collect::<std::io::Result<_>>()?;
        Ok(Self::from_raw_box(rows))
    }
}

pub struct FirstMoveSearcher {
    first_move: NodeMemberPointer<u64>,
    g: NodeMemberPointer<f64>,
}

impl FirstMoveSearcher {
    pub fn new(builder: &mut NodeBuilder) -> Self {
        FirstMoveSearcher {
            first_move: builder.add_field(0),
            g: builder.add_field(f64::INFINITY),
        }
    }

    pub fn g(&self) -> NodeMemberPointer<f64> {
        self.g
    }

    pub fn search<'a, Exp, Edge, Open>(
        &mut self,
        start: NodeRef<'a>,
        mut expander: Exp,
        mut open: Open,
        mut found: impl FnMut(NodeRef<'a>, u64),
    ) where
        Exp: Expander<'a, Edge = Edge>,
        Edge: Successor<'a> + Cost + EdgeId,
        Open: OpenList<'a>,
    {
        let FirstMoveSearcher { first_move, g } = *self;

        start.set(g, 0.0);

        let mut edges = vec![];

        // We need to handle expansion of the start node specially so that we can set the first
        // move set correctly.
        expander.expand(start, &mut edges);
        for edge in &edges {
            let node = edge.successor();
            let edge_id = edge.edge_id();
            assert!(
                edge_id < 63,
                "edge id {edge_id} exceeds maximum supported value 62"
            );
            node.set(g, edge.cost());
            node.set(first_move, 1 << edge.edge_id());
            node.set_parent(Some(start));
            open.relaxed(node);
        }

        while let Some(node) = open.next() {
            found(node, node.get(first_move));
            edges.clear();
            expander.expand(node, &mut edges);

            let node_g = node.get(g);
            let node_first_move = node.get(first_move);

            for edge in &edges {
                let successor = edge.successor();
                let new_g = edge.cost() + node_g;
                // TODO: think about floating point round-off error
                if new_g < successor.get(g) {
                    // Shorter path to node; update g and first move field.
                    successor.set(g, new_g);
                    successor.set(first_move, node_first_move);
                    successor.set_parent(Some(node));
                    open.relaxed(successor);
                } else if new_g == successor.get(g) {
                    // In case of tie, multiple first moves may allow optimal paths.
                    successor.set(first_move, successor.get(first_move) | node_first_move);
                }
            }
        }
    }
}

/// Re-orders the array into Eytzinger order, allowing slightly faster lookup than binary search.
fn reorder_eytzinger(items: &mut impl Iterator<Item = CpdEntry>, into: &mut [CpdEntry], k: usize) {
    if k < into.len() {
        reorder_eytzinger(items, into, 2 * k + 1);
        into[k] = items.next().unwrap();
        reorder_eytzinger(items, into, 2 * k + 2);
    }
}
