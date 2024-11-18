use enumset::EnumSet;
use mkpath_core::traits::{Expander, NodePool, OpenList};
use mkpath_core::{NodeBuilder, NodeMemberPointer};
use mkpath_grid::bucket_queue::BucketQueueFactory;
use mkpath_grid::{BitGrid, Direction, GridPool};
use mkpath_jps::{canonical_successors, CanonicalGridExpander};

pub struct FirstMoveComputer<'a> {
    map: &'a BitGrid,
    pool: GridPool,
    pqueue: BucketQueueFactory,
    state: NodeMemberPointer<(i32, i32)>,
    g: NodeMemberPointer<f64>,
    successors: NodeMemberPointer<EnumSet<Direction>>,
    first_move: NodeMemberPointer<EnumSet<Direction>>,
}

impl<'a> FirstMoveComputer<'a> {
    pub fn new(map: &'a BitGrid) -> Self {
        let mut builder = NodeBuilder::new();
        let state = builder.add_field((-1, -1));
        let successors = builder.add_field(EnumSet::all());
        let first_move = builder.add_field(EnumSet::all());
        let g = builder.add_field(f64::INFINITY);
        let pqueue = BucketQueueFactory::new(&mut builder);
        let pool = GridPool::new(builder.build(), state, map.width(), map.height());

        FirstMoveComputer {
            map,
            pool,
            pqueue,
            state,
            g,
            successors,
            first_move,
        }
    }

    pub fn compute(
        &mut self,
        source: (i32, i32),
        mut fm_cb: impl FnMut((i32, i32), EnumSet<Direction>),
    ) {
        let FirstMoveComputer {
            map,
            ref mut pool,
            ref mut pqueue,
            state,
            g,
            successors,
            first_move,
        } = *self;
        pool.reset();

        let mut edges = vec![];
        let mut expander = CanonicalGridExpander::new(&map, pool, state);
        let mut open = pqueue.new_queue(g, 0.999);

        let start_node = pool.generate(source);
        start_node.set(g, 0.0);

        expander.expand(start_node, &mut edges);
        for edge in &edges {
            let node = edge.successor;
            node.set(g, edge.cost);
            node.set(first_move, EnumSet::only(edge.direction));
            node.set_parent(Some(start_node));
            let (x, y) = node.get(state);
            node.set(
                successors,
                canonical_successors(map.get_neighborhood(x, y), Some(edge.direction)),
            );
            open.relaxed(node);
        }

        while let Some(node) = open.next() {
            fm_cb(node.get(state), node.get(first_move));
            edges.clear();
            unsafe {
                expander.expand_unchecked(node, &mut edges, node.get(successors));
            }

            let node_g = node.get(g);
            let node_first_move = node.get(first_move);

            for edge in &edges {
                let successor = edge.successor;
                let (x, y) = successor.get(state);
                let new_g = edge.cost + node_g;
                // TODO: think about floating point round-off error
                if new_g < successor.get(g) {
                    // Shorter path to node; overwrite first move and successors.
                    successor.set(g, new_g);
                    successor.set(first_move, node_first_move);
                    successor.set(
                        successors,
                        canonical_successors(map.get_neighborhood(x, y), Some(edge.direction)),
                    );
                    successor.set_parent(Some(node));
                    open.relaxed(successor);
                } else if new_g == successor.get(g) {
                    // In case of tie, multiple first moves may allow optimal paths.
                    // Additionally, there are more canonical successors to consider
                    // when the node is expanded.
                    successor.set(first_move, successor.get(first_move) | node_first_move);
                    successor.set(
                        successors,
                        successor.get(successors)
                            | canonical_successors(
                                map.get_neighborhood(x, y),
                                Some(edge.direction),
                            ),
                    );
                }
            }
        }
    }
}
