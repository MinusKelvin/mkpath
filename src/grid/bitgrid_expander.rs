use std::f64::consts::SQRT_2;

use crate::node::NodeRef;

use super::{BitGrid, GridPool};

pub struct BitGridExpander<'a> {
    map: &'a BitGrid,
    node_pool: &'a GridPool,
}

impl<'a> BitGridExpander<'a> {
    pub fn new(map: &'a BitGrid, node_pool: &'a GridPool) -> Self {
        assert!(
            node_pool.width() >= map.width(),
            "node pool must be wide enough for the map"
        );
        assert!(
            node_pool.height() >= map.height(),
            "node pool must be tall enough for the map"
        );

        BitGridExpander { map, node_pool }
    }

    pub fn expand(&mut self, node: NodeRef, edges: &mut Vec<(NodeRef<'a>, f64)>) {
        let (x, y) = node.get(self.node_pool.state_member());
        assert!(
            self.map.get(x, y),
            "attempt to expand node at untraversable location"
        );

        unsafe {
            let north_traversable = self.map.get_unchecked(x, y - 1);
            if north_traversable {
                edges.push((self.node_pool.generate_unchecked(x, y - 1), 1.0));
            }

            let south_traversable = self.map.get_unchecked(x, y + 1);
            if south_traversable {
                edges.push((self.node_pool.generate_unchecked(x, y + 1), 1.0));
            }

            if self.map.get_unchecked(x - 1, y) {
                edges.push((self.node_pool.generate_unchecked(x - 1, y), 1.0));

                if north_traversable && self.map.get_unchecked(x - 1, y - 1) {
                    edges.push((self.node_pool.generate_unchecked(x - 1, y - 1), SQRT_2));
                }

                if south_traversable && self.map.get_unchecked(x - 1, y + 1) {
                    edges.push((self.node_pool.generate_unchecked(x - 1, y + 1), SQRT_2));
                }
            }
            if self.map.get_unchecked(x + 1, y) {
                edges.push((self.node_pool.generate_unchecked(x + 1, y), 1.0));

                if north_traversable && self.map.get_unchecked(x + 1, y - 1) {
                    edges.push((self.node_pool.generate_unchecked(x + 1, y - 1), SQRT_2));
                }

                if south_traversable && self.map.get_unchecked(x + 1, y + 1) {
                    edges.push((self.node_pool.generate_unchecked(x + 1, y + 1), SQRT_2));
                }
            }
        }
    }
}
