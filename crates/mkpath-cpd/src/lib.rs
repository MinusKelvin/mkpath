use std::io::{Read, Write};

use mkpath_core::traits::{Cost, EdgeId, Expander, OpenList, Successor};
use mkpath_core::{NodeBuilder, NodeMemberPointer, NodeRef, PriorityQueueFactory};

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
    let mut stack = vec![vec![]];
    found(start);
    expander.expand(start, &mut stack[0]);

    while let Some(edges) = stack.last_mut() {
        if let Some(edge) = edges.pop() {
            let node = edge.successor();
            if found(node) {
                let mut new_edges = vec![];
                expander.expand(node, &mut new_edges);
                stack.push(new_edges);
            }
        } else {
            stack.pop();
        }
    }
}

pub struct CpdRow {
    runs: Box<[CpdEntry]>,
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
    pub fn compute<'a, M, S, E, Edge>(
        mapper: &M,
        searcher: &mut FirstMoveSearcher,
        expander: E,
        start: NodeRef<'a>,
        state: NodeMemberPointer<S>,
    ) -> Self
    where
        S: Copy + 'static,
        M: StateIdMapper<State = S>,
        E: Expander<'a, Edge = Edge>,
        Edge: Successor<'a> + Cost + EdgeId,
    {
        assert!(mapper.num_ids() < 1 << 26);
        let mut first_moves = vec![!0; mapper.num_ids()];

        searcher.search(start, expander, |node, fm| {
            first_moves[mapper.state_to_id(node.get(state))] = fm
        });

        let mut runs = vec![];
        let mut current_id = 0;
        let mut current_moves = !0;
        for (id, moves) in first_moves.into_iter().enumerate().chain(Some((0, 0))) {
            if current_moves & moves == 0 {
                runs.push(CpdEntry(current_id | current_moves.trailing_zeros() << 26));
                current_id = id as u32;
                current_moves = moves;
            } else {
                current_moves &= moves;
            }
        }

        CpdRow {
            runs: runs.into_boxed_slice(),
        }
    }

    pub fn lookup(&self, id: usize) -> usize {
        let index = match self.runs.binary_search_by_key(&id, |run| run.start()) {
            Ok(idx) => idx,
            Err(idx) => idx - 1,
        };
        self.runs[index].edge()
    }

    pub fn save(&self, to: &mut impl Write) -> std::io::Result<()> {
        to.write_all(&(self.runs.len() as u32).to_le_bytes())?;
        for &run in &*self.runs {
            to.write_all(&run.0.to_le_bytes())?;
        }
        Ok(())
    }

    pub fn load(from: &mut impl Read) -> std::io::Result<Self> {
        let mut bytes = [0; 4];
        from.read_exact(&mut bytes)?;
        let len = u32::from_le_bytes(bytes) as usize;
        Ok(CpdRow {
            runs: (0..len)
                .map(|_| {
                    from.read_exact(&mut bytes)?;
                    Ok(CpdEntry(u32::from_le_bytes(bytes)))
                })
                .collect::<std::io::Result<_>>()?,
        })
    }
}

pub struct FirstMoveSearcher {
    first_move: NodeMemberPointer<u64>,
    g: NodeMemberPointer<f64>,
    pqueue: PriorityQueueFactory,
}

impl FirstMoveSearcher {
    pub fn new(builder: &mut NodeBuilder) -> Self {
        FirstMoveSearcher {
            first_move: builder.add_field(0),
            g: builder.add_field(f64::INFINITY),
            pqueue: PriorityQueueFactory::new(builder),
        }
    }

    pub fn search<'a, E, Edge>(
        &mut self,
        start: NodeRef<'a>,
        mut expander: E,
        mut found: impl FnMut(NodeRef<'a>, u64),
    ) where
        E: Expander<'a, Edge = Edge>,
        Edge: Successor<'a> + Cost + EdgeId,
    {
        let first_move = self.first_move;
        let g = self.g;

        start.set(g, 0.0);

        let mut open = self.pqueue.new_queue(g);
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
            open.relaxed(node);
        }

        while let Some(node) = open.next() {
            found(node, node.get(first_move));
            edges.clear();
            expander.expand(node, &mut edges);
            for edge in &edges {
                let successor = edge.successor();
                let new_g = edge.cost() + node.get(g);
                // TODO: think about floating point round-off error
                if new_g < successor.get(g) {
                    // Shorter path to node; update g and first move field.
                    successor.set(g, new_g);
                    successor.set(first_move, node.get(first_move));
                    open.relaxed(successor);
                } else if new_g == successor.get(g) {
                    // In case of tie, multiple first moves may allow optimal paths.
                    // successor.set(first_move, successor.get(first_move) | node.get(first_move));
                }
            }
        }
    }
}
