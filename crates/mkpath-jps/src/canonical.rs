use std::f64::consts::SQRT_2;

use mkpath_core::traits::Expander;
use mkpath_core::NodeRef;
use mkpath_grid::{BitGrid, Direction, GridEdge, GridStateMapper};

use crate::canonical_successors;

pub struct CanonicalGridExpander<'a, P> {
    node_pool: &'a P,
    map: &'a BitGrid,
}

impl<'a, P: GridStateMapper> CanonicalGridExpander<'a, P> {
    pub fn new(map: &'a BitGrid, node_pool: &'a P) -> Self {
        CanonicalGridExpander { node_pool, map }
    }
}

impl<'a, P: GridStateMapper> Expander<'a> for CanonicalGridExpander<'a, P> {
    type Edge = GridEdge<'a>;

    fn expand(&mut self, node: NodeRef<'a>, edges: &mut Vec<Self::Edge>) {
        let (x, y) = node.get(self.node_pool.state_member());

        let dir = node.get_parent().and_then(|parent| {
            let (px, py) = parent.get(self.node_pool.state_member());
            crate::reached_direction((px, py), (x, y))
        });

        let successors = canonical_successors(self.map.get_neighborhood(x, y), dir);

        unsafe {
            // All nodes have the traversability of the relevant tile checked via successor set.
            // Remaining preconditions hold trivially.

            if successors.contains(Direction::North) {
                edges.push(GridEdge {
                    successor: self.node_pool.generate_unchecked((x, y - 1)),
                    cost: 1.0,
                    direction: Direction::North,
                });
            }
            if successors.contains(Direction::West) {
                edges.push(GridEdge {
                    successor: self.node_pool.generate_unchecked((x - 1, y)),
                    cost: 1.0,
                    direction: Direction::West,
                });
            }
            if successors.contains(Direction::South) {
                edges.push(GridEdge {
                    successor: self.node_pool.generate_unchecked((x, y + 1)),
                    cost: 1.0,
                    direction: Direction::South,
                });
            }
            if successors.contains(Direction::East) {
                edges.push(GridEdge {
                    successor: self.node_pool.generate_unchecked((x + 1, y)),
                    cost: 1.0,
                    direction: Direction::East,
                });
            }
            if successors.contains(Direction::NorthWest) {
                edges.push(GridEdge {
                    successor: self.node_pool.generate_unchecked((x - 1, y - 1)),
                    cost: SQRT_2,
                    direction: Direction::NorthWest,
                });
            }
            if successors.contains(Direction::SouthWest) {
                edges.push(GridEdge {
                    successor: self.node_pool.generate_unchecked((x - 1, y + 1)),
                    cost: SQRT_2,
                    direction: Direction::SouthWest,
                });
            }
            if successors.contains(Direction::SouthEast) {
                edges.push(GridEdge {
                    successor: self.node_pool.generate_unchecked((x + 1, y + 1)),
                    cost: SQRT_2,
                    direction: Direction::SouthEast,
                });
            }
            if successors.contains(Direction::NorthEast) {
                edges.push(GridEdge {
                    successor: self.node_pool.generate_unchecked((x + 1, y - 1)),
                    cost: SQRT_2,
                    direction: Direction::NorthEast,
                });
            }
        }
    }
}
