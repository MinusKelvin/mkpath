use mkpath_core::traits::{Expander, WeightedEdge};
use mkpath_core::NodeRef;
use mkpath_grid::{BitGrid, GridStateMapper, SAFE_SQRT_2};

use crate::{canonical_successors, Direction, JumpDatabase};

/// Jump Point Search Plus expander.
///
/// Harabor, D., & Grastien, A. (2014, May). Improving jump point search. In Proceedings of the
/// International Conference on Automated Planning and Scheduling (Vol. 24, pp. 128-135).
pub struct JpsPlusExpander<'a, P> {
    node_pool: &'a P,
    map: &'a BitGrid,
    jump_db: &'a JumpDatabase,
    target: (i32, i32),
}

impl<'a, P: GridStateMapper> JpsPlusExpander<'a, P> {
    pub fn new(
        map: &'a BitGrid,
        jump_db: &'a JumpDatabase,
        node_pool: &'a P,
        target: (i32, i32),
    ) -> Self {
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

        // Establish invariant that coordinates in-bounds of the map are in-bounds of the jump
        // database, and vice-versa.
        // We don't check that the content of the jump database is actually correct for the map
        // since that's a) slow b) merely a logic error; not required for safety.
        assert_eq!(
            map.width(),
            jump_db.width(),
            "jump database has incorrect width"
        );
        assert_eq!(
            map.height(),
            jump_db.height(),
            "jump database has incorrect height"
        );

        JpsPlusExpander {
            node_pool,
            map,
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
            cost += dist as f64 * SAFE_SQRT_2;

            if (x, y) == self.target {
                edges.push(WeightedEdge {
                    successor: self.node_pool.generate_unchecked((x, y)),
                    cost,
                });
                break;
            }

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

        let successors = canonical_successors(self.map.get_neighborhood(x, y), dir);

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
