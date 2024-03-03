use crate::NodeRef;

pub trait Expander<'a> {
    type Edge: 'a;

    fn expand(&mut self, node: NodeRef<'a>, edges: &mut Vec<Self::Edge>);
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