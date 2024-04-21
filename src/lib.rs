use mkpath_core::traits::{Cost, Expander, OpenList, Successor};
pub use mkpath_core::*;
pub use mkpath_cpd as cpd;
pub use mkpath_grid as grid;
pub use mkpath_jps as jps;
pub use mkpath_grid_gb as grid_gb;

pub struct AStarSearcher {
    g: NodeMemberPointer<f64>,
    h: NodeMemberPointer<f64>,
    f: NodeMemberPointer<f64>,
}

impl AStarSearcher {
    pub fn new(builder: &mut NodeBuilder) -> Self {
        let g = builder.add_field(f64::INFINITY);
        let h = builder.add_field(f64::NAN);
        let f = builder.add_field(f64::INFINITY);
        AStarSearcher { g, h, f }
    }

    pub fn g(&self) -> NodeMemberPointer<f64> {
        self.g
    }

    pub fn ordering(&self) -> impl FieldComparator {
        (self.f, self.h)
    }

    pub fn search<'a, Exp, Open, Edge>(
        &mut self,
        mut expander: Exp,
        mut open_list: Open,
        mut heuristic: impl FnMut(NodeRef<'a>) -> f64,
        mut goal_test: impl FnMut(NodeRef<'a>) -> bool,
        start: NodeRef<'a>,
    ) -> Option<Vec<NodeRef<'a>>>
    where
        Exp: Expander<'a, Edge = Edge>,
        Edge: Successor<'a> + Cost,
        Open: OpenList<'a>,
    {
        let AStarSearcher { g, h, f } = *self;

        let mut edges = vec![];

        start.set(g, 0.0);
        start.set(h, heuristic(start));
        start.set(f, start.get(h));
        open_list.relaxed(start);

        while let Some(node) = open_list.next() {
            if goal_test(node) {
                let mut path = vec![node];
                while let Some(parent) = path[path.len() - 1].get_parent() {
                    path.push(parent);
                }
                path.reverse();
                return Some(path);
            }

            edges.clear();
            expander.expand(node, &mut edges);

            let node_g = node.get(g);

            for edge in &edges {
                let successor = edge.successor();
                let new_g = node_g + edge.cost();
                if new_g < successor.get(g) {
                    if successor.get(h).is_nan() {
                        successor.set(h, heuristic(successor));
                    }
                    successor.set(g, new_g);
                    successor.set(f, new_g + successor.get(h));
                    successor.set_parent(Some(node));
                    open_list.relaxed(successor);
                }
            }
        }

        None
    }
}
