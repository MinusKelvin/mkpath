//! Types and utilities for working with 8-connected grid maps.

use mkpath_core::traits::Expander;
use mkpath_core::{NodeAllocator, NodeBuilder, NodeMemberPointer, NodeRef};
use mkpath_ess::ExplicitStateSpace;

use crate::{BitGrid, Direction, Grid, GridEdge, GridNodePool, GridPool, SAFE_SQRT_2};

pub struct EightConnectedExpander<'s, 'a, P> {
    map: &'a BitGrid,
    node_pool: &'s P,
    state: NodeMemberPointer<(i32, i32)>,
}

impl<'s, 'a, P: GridNodePool> EightConnectedExpander<'s, 'a, P> {
    pub fn new(map: &'a BitGrid, node_pool: &'s P, state: NodeMemberPointer<(i32, i32)>) -> Self {
        // Establish invariant that coordinates in-bounds of the map are also in-bounds of the
        // node pool.
        assert!(
            node_pool.width() >= map.width(),
            "node pool must be wide enough for the map"
        );
        assert!(
            node_pool.height() >= map.height(),
            "node pool must be tall enough for the map"
        );

        EightConnectedExpander {
            map,
            node_pool,
            state,
        }
    }
}

impl<'s, 'a, P: GridNodePool> Expander<'s> for EightConnectedExpander<'s, 'a, P> {
    type Edge = GridEdge<'s>;

    fn expand(&mut self, node: NodeRef<'s>, edges: &mut Vec<GridEdge<'s>>) {
        let (x, y) = node.get(self.state);

        assert!(
            self.map.get(x, y),
            "attempt to expand node at untraversable location"
        );

        unsafe {
            // Since x, y is traversable, these are all padded in-bounds, as required by
            // get_unchecked.
            // Since the various offsets for which nodes are generated are verified to be
            // traversable, we know that the offset coordinate is in-bounds of the map, and
            // therefore is also in-bounds of the node pool.

            let north_traversable = self.map.get_unchecked(x, y - 1);
            if north_traversable {
                edges.push(GridEdge {
                    successor: self.node_pool.generate_unchecked((x, y - 1)),
                    cost: 1.0,
                    direction: Direction::North,
                });
            }

            let south_traversable = self.map.get_unchecked(x, y + 1);
            if south_traversable {
                edges.push(GridEdge {
                    successor: self.node_pool.generate_unchecked((x, y + 1)),
                    cost: 1.0,
                    direction: Direction::South,
                });
            }

            if self.map.get_unchecked(x - 1, y) {
                edges.push(GridEdge {
                    successor: self.node_pool.generate_unchecked((x - 1, y)),
                    cost: 1.0,
                    direction: Direction::West,
                });

                if north_traversable && self.map.get_unchecked(x - 1, y - 1) {
                    edges.push(GridEdge {
                        successor: self.node_pool.generate_unchecked((x - 1, y - 1)),
                        cost: SAFE_SQRT_2,
                        direction: Direction::NorthWest,
                    });
                }

                if south_traversable && self.map.get_unchecked(x - 1, y + 1) {
                    edges.push(GridEdge {
                        successor: self.node_pool.generate_unchecked((x - 1, y + 1)),
                        cost: SAFE_SQRT_2,
                        direction: Direction::SouthWest,
                    });
                }
            }
            if self.map.get_unchecked(x + 1, y) {
                edges.push(GridEdge {
                    successor: self.node_pool.generate_unchecked((x + 1, y)),
                    cost: 1.0,
                    direction: Direction::East,
                });

                if north_traversable && self.map.get_unchecked(x + 1, y - 1) {
                    edges.push(GridEdge {
                        successor: self.node_pool.generate_unchecked((x + 1, y - 1)),
                        cost: SAFE_SQRT_2,
                        direction: Direction::NorthEast,
                    });
                }

                if south_traversable && self.map.get_unchecked(x + 1, y + 1) {
                    edges.push(GridEdge {
                        successor: self.node_pool.generate_unchecked((x + 1, y + 1)),
                        cost: SAFE_SQRT_2,
                        direction: Direction::SouthEast,
                    });
                }
            }
        }
    }
}

pub fn octile_distance(from: (i32, i32), to: (i32, i32)) -> f64 {
    let dx = (from.0 - to.0).abs();
    let dy = (from.1 - to.1).abs();
    let diagonals = dx.min(dy);
    let orthos = dx.max(dy) - diagonals;
    orthos as f64 + diagonals as f64 * SAFE_SQRT_2
}

#[repr(transparent)]
pub struct EightConnectedDomain(pub BitGrid);

impl EightConnectedDomain {
    pub fn from_ref(value: &BitGrid) -> &Self {
        // SAFETY: EightConnectedDomain has the same representation as BitGrid
        //         (transparent repr), so punning the referent type is safe.
        unsafe { &*(value as *const _ as *const _) }
    }
}

impl ExplicitStateSpace for EightConnectedDomain {
    type State = (i32, i32);

    type Auxiliary<T> = Grid<T>;

    type NodePool = GridPool;

    type Expander<'s> = EightConnectedExpander<'s, 's, Self::NodePool>
    where
        Self: 's;

    fn new_auxiliary<T>(&self, mut init: impl FnMut(Self::State) -> T) -> Self::Auxiliary<T> {
        Grid::new(self.0.width(), self.0.height(), |x, y| init((x, y)))
    }

    fn add_state_field(&self, builder: &mut NodeBuilder) -> NodeMemberPointer<Self::State> {
        builder.add_field((-1, -1))
    }

    fn new_node_pool(
        &self,
        alloc: NodeAllocator,
        state: NodeMemberPointer<Self::State>,
    ) -> Self::NodePool {
        GridPool::new(alloc, state, self.0.width(), self.0.height())
    }

    fn new_expander<'a>(
        &'a self,
        node_pool: &'a Self::NodePool,
        state: NodeMemberPointer<Self::State>,
    ) -> Self::Expander<'a> {
        EightConnectedExpander::new(&self.0, node_pool, state)
    }

    fn list_valid_states(&self) -> Vec<Self::State> {
        let mut res = vec![];
        for y in 0..self.0.height() {
            for x in 0..self.0.width() {
                if self.0.get(x, y) {
                    res.push((x, y));
                }
            }
        }
        res
    }
}
