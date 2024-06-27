mod differential;

pub use differential::DifferentialHeuristic;
use mkpath_core::traits::{Cost, Expander, NodePool, OpenList, Successor};
use mkpath_core::{NodeBuilder, NodeMemberPointer, PriorityQueueFactory};
use mkpath_ess::ExplicitStateSpace;

struct Searcher<SS: ExplicitStateSpace> {
    node_pool: SS::NodePool,
    pqueue_factory: PriorityQueueFactory,
    state: NodeMemberPointer<SS::State>,
    g: NodeMemberPointer<f64>,
}

impl<SS: ExplicitStateSpace> Searcher<SS>
where
    for<'a> <SS::Expander<'a> as Expander<'a>>::Edge: Successor<'a> + Cost,
{
    fn new(domain: &SS, nodes_required: usize) -> Self {
        let mut builder = NodeBuilder::new();
        let state = domain.add_state_field(&mut builder);
        let g = builder.add_field(f64::INFINITY);
        let pqueue_factory = PriorityQueueFactory::new(&mut builder);
        let node_pool = domain.new_node_pool(builder.build_with_capacity(nodes_required), state);

        Searcher {
            node_pool,
            pqueue_factory,
            state,
            g,
        }
    }

    fn search(&mut self, domain: &SS, start: SS::State, mut f: impl FnMut(SS::State, f64)) {
        let Self {
            ref mut node_pool,
            ref mut pqueue_factory,
            state,
            g,
        } = *self;

        node_pool.reset();

        let mut expander = domain.new_expander(node_pool, state);
        let mut pqueue = pqueue_factory.new_queue(g);
        let mut edges = vec![];

        let start = node_pool.generate(start);
        start.set(g, 0.0);
        pqueue.relaxed(start);

        while let Some(node) = pqueue.next() {
            let node_g = node.get(g);
            f(node.get(state), node_g);

            edges.clear();
            expander.expand(node, &mut edges);

            for edge in &edges {
                let successor = edge.successor();
                let new_g = node_g + edge.cost();
                if new_g < successor.get(g) {
                    successor.set(g, new_g);
                    successor.set_parent(Some(node));
                    pqueue.relaxed(successor);
                }
            }
        }
    }
}
