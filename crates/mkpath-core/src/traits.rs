use crate::NodeRef;

pub trait Expander<'a> {
    type Edge: 'a;

    fn expand(&mut self, node: NodeRef<'a>, edges: &mut Vec<Self::Edge>);
}

pub trait OpenList<'a> {
    fn next(&mut self) -> Option<NodeRef<'a>>;

    fn relaxed(&mut self, node: NodeRef<'a>);
}

pub trait NodePool {
    type State;

    fn reset(&mut self);

    fn generate(&self, state: Self::State) -> NodeRef;
}

pub trait Successor<'a> {
    fn successor(&self) -> NodeRef<'a>;
}

pub trait Cost {
    fn cost(&self) -> f64;
}

pub trait EdgeId {
    fn edge_id(&self) -> usize;
}

pub struct WeightedEdge<'a> {
    pub successor: NodeRef<'a>,
    pub cost: f64,
}

impl<'a> Successor<'a> for WeightedEdge<'a> {
    fn successor(&self) -> NodeRef<'a> {
        self.successor
    }
}

impl Cost for WeightedEdge<'_> {
    fn cost(&self) -> f64 {
        self.cost
    }
}
