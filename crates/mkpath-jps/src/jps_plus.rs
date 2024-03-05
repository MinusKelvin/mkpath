use std::f64::consts::SQRT_2;

use mkpath_core::traits::{Expander, WeightedEdge};
use mkpath_core::NodeRef;
use mkpath_grid::GridStateMapper;

use crate::{canonical_successors, Direction, JumpDatabase};

/// Jump Point Search Plus expander.
///
/// Harabor, D., & Grastien, A. (2014, May). Improving jump point search. In Proceedings of the
/// International Conference on Automated Planning and Scheduling (Vol. 24, pp. 128-135).
pub struct JpsPlusExpander<'a, P> {
    node_pool: &'a P,
    jump_db: &'a JumpDatabase,
    target: (i32, i32),
}

impl<'a, P: GridStateMapper> JpsPlusExpander<'a, P> {
    pub fn new(jump_db: &'a JumpDatabase, node_pool: &'a P, target: (i32, i32)) -> Self {
        JpsPlusExpander {
            node_pool,
            jump_db,
            target,
        }
    }

    #[inline(always)]
    unsafe fn jump_ortho(
        &self,
        x: i32,
        y: i32,
        dir: Direction,
        cost: f64,
        edges: &mut Vec<WeightedEdge<'a>>,
    ) {
        let (dx, dy) = match dir {
            Direction::North => (0, -1),
            Direction::West => (-1, 0),
            Direction::South => (0, 1),
            Direction::East => (1, 0),
            _ => unreachable!(),
        };

        if let Some(dist) = self.jump_db.ortho_jump_unchecked(x, y, dir, self.target) {
            edges.push(WeightedEdge {
                successor: self
                    .node_pool
                    .generate_unchecked((x + dx * dist, y + dy * dist)),
                cost: cost + dist as f64,
            })
        }
    }

    #[inline(always)]
    unsafe fn jump_diagonal(
        &self,
        mut x: i32,
        mut y: i32,
        dir: Direction,
        edges: &mut Vec<WeightedEdge<'a>>,
    ) {
        let (dx, dy, dir_x, dir_y) = match dir {
            Direction::NorthWest => (-1, -1, Direction::West, Direction::North),
            Direction::SouthWest => (-1, 1, Direction::West, Direction::South),
            Direction::SouthEast => (1, 1, Direction::East, Direction::South),
            Direction::NorthEast => (1, -1, Direction::East, Direction::North),
            _ => unreachable!(),
        };

        let mut cost = 0.0;
        while let Some((dist, _)) = self.jump_db.diagonal_jump_unchecked(x, y, dir, self.target) {
            x += dx * dist;
            y += dy * dist;
            cost += dist as f64 * SQRT_2;

            self.jump_ortho(x, y, dir_x, cost, edges);
            self.jump_ortho(x, y, dir_y, cost, edges);
        }
    }
}

impl<'a, P: GridStateMapper> Expander<'a> for JpsPlusExpander<'a, P> {
    type Edge = WeightedEdge<'a>;

    fn expand(&mut self, node: NodeRef<'a>, edges: &mut Vec<Self::Edge>) {
        let (x, y) = node.get(self.node_pool.state_member());

        let dir = node.get_parent().and_then(|parent| {
            let (px, py) = parent.get(self.node_pool.state_member());
            crate::reached_direction((px, py), (x, y))
        });

        let successors = canonical_successors(self.jump_db.map().get_neighborhood(x, y), dir);

        unsafe {
            // All jumps have the traversability of the relevant tile checked via successor set.
            // Remaining preconditions hold trivially.

            if successors.contains(Direction::North) {
                self.jump_ortho(x, y, Direction::North, 0.0, edges);
            }
            if successors.contains(Direction::West) {
                self.jump_ortho(x, y, Direction::West, 0.0, edges);
            }
            if successors.contains(Direction::South) {
                self.jump_ortho(x, y, Direction::South, 0.0, edges);
            }
            if successors.contains(Direction::East) {
                self.jump_ortho(x, y, Direction::East, 0.0, edges);
            }
            if successors.contains(Direction::NorthWest) {
                self.jump_diagonal(x, y, Direction::NorthWest, edges);
            }
            if successors.contains(Direction::SouthWest) {
                self.jump_diagonal(x, y, Direction::SouthWest, edges);
            }
            if successors.contains(Direction::SouthEast) {
                self.jump_diagonal(x, y, Direction::SouthEast, edges);
            }
            if successors.contains(Direction::NorthEast) {
                self.jump_diagonal(x, y, Direction::NorthEast, edges);
            }
        }
    }
}
