use mkpath_core::traits::Expander;
use mkpath_core::{HashPool, NodeBuilder, NodeMemberPointer};
use mkpath_grid::{octile_distance, BitGrid, Direction};
use mkpath_jps::{canonical_successors, reached_direction, JumpDatabase};

use crate::{PartialCellCpd, TopsExpander};

pub struct ToppingPlus<'a> {
    map: &'a BitGrid,
    jump_db: &'a JumpDatabase,
    cpd: &'a PartialCellCpd,
    node_pool: HashPool<(i32, i32)>,
    cost: NodeMemberPointer<f64>,
}

impl<'a> ToppingPlus<'a> {
    pub fn new(map: &'a BitGrid, jump_db: &'a JumpDatabase, cpd: &'a PartialCellCpd) -> Self {
        let mut builder = NodeBuilder::new();
        let state = builder.add_field((-1, -1));
        let cost = builder.add_field(f64::INFINITY);

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

        ToppingPlus {
            map,
            jump_db,
            cpd,
            node_pool: HashPool::new(builder.build(), state),
            cost,
        }
    }

    pub fn get_path(&mut self, start: (i32, i32), target: (i32, i32)) -> (Vec<(i32, i32)>, f64) {
        self.node_pool.reset();

        let cost = self.cost;
        let state = self.node_pool.state_member();

        let start_node = self.node_pool.generate(start);
        let target_node = self.node_pool.generate(target);
        target_node.set(cost, 0.0);

        let mut starts = vec![];
        TopsExpander::new(self.map, self.jump_db, self.cpd, &self.node_pool, target)
            .expand(start_node, &mut starts);

        let mut node_stack = vec![];

        for edge in &starts {
            if edge.successor.ptr_eq(target_node) {
                return (vec![start, target], edge.cost);
            }
        }

        'start_successor: for edge in starts {
            let mut current_node = edge.successor;
            let mut prev_state = start;
            node_stack.clear();

            while current_node.get(cost).is_infinite() {
                let state = current_node.get(state);
                let going = reached_direction(prev_state, state);
                let canonical =
                    canonical_successors(self.map.get_neighborhood(state.0, state.1), going);

                let dir = self
                    .cpd
                    .query(state, target)
                    .expect("cpd did not have move for jump point");

                if !canonical.contains(dir) {
                    continue 'start_successor;
                }

                let next_state = match dir {
                    Direction::North => unsafe {
                        // SAFETY: We know that TopsExpander produces successors whose
                        //         coordinates are in-bounds, and we know that jumping with the
                        //         jump distance database gives us coordinates that are in-bounds,
                        //         so state will always be in-bounds. Similar for below calls.
                        let dist = self
                            .jump_db
                            .ortho_jump_unchecked(state.0, state.1, Direction::North, target)
                            .unwrap();
                        (state.0, state.1 - dist)
                    },
                    Direction::West => unsafe {
                        let dist = self
                            .jump_db
                            .ortho_jump_unchecked(state.0, state.1, Direction::West, target)
                            .unwrap();
                        (state.0 - dist, state.1)
                    },
                    Direction::South => unsafe {
                        let dist = self
                            .jump_db
                            .ortho_jump_unchecked(state.0, state.1, Direction::South, target)
                            .unwrap();
                        (state.0, state.1 + dist)
                    },
                    Direction::East => unsafe {
                        let dist = self
                            .jump_db
                            .ortho_jump_unchecked(state.0, state.1, Direction::East, target)
                            .unwrap();
                        (state.0 + dist, state.1)
                    },
                    Direction::NorthWest => unsafe {
                        let (dist, turn) = self
                            .jump_db
                            .diagonal_jump_unchecked(state.0, state.1, Direction::NorthWest, target)
                            .unwrap();
                        let (x, y) = (state.0 - dist, state.1 - dist);
                        match turn {
                            Some((Direction::North, dist2)) => (x, y - dist2),
                            Some((Direction::West, dist2)) => (x - dist2, y),
                            None => (x, y),
                            _ => unreachable!(),
                        }
                    },
                    Direction::SouthWest => unsafe {
                        let (dist, turn) = self
                            .jump_db
                            .diagonal_jump_unchecked(state.0, state.1, Direction::SouthWest, target)
                            .unwrap();
                        let (x, y) = (state.0 - dist, state.1 + dist);
                        match turn {
                            Some((Direction::South, dist2)) => (x, y + dist2),
                            Some((Direction::West, dist2)) => (x - dist2, y),
                            None => (x, y),
                            _ => unreachable!(),
                        }
                    },
                    Direction::SouthEast => unsafe {
                        let (dist, turn) = self
                            .jump_db
                            .diagonal_jump_unchecked(state.0, state.1, Direction::SouthEast, target)
                            .unwrap();
                        let (x, y) = (state.0 + dist, state.1 + dist);
                        match turn {
                            Some((Direction::South, dist2)) => (x, y + dist2),
                            Some((Direction::East, dist2)) => (x + dist2, y),
                            None => (x, y),
                            _ => unreachable!(),
                        }
                    },
                    Direction::NorthEast => unsafe {
                        let (dist, turn) = self
                            .jump_db
                            .diagonal_jump_unchecked(state.0, state.1, Direction::NorthEast, target)
                            .unwrap();
                        let (x, y) = (state.0 + dist, state.1 - dist);
                        match turn {
                            Some((Direction::North, dist2)) => (x, y - dist2),
                            Some((Direction::East, dist2)) => (x + dist2, y),
                            None => (x, y),
                            _ => unreachable!(),
                        }
                    },
                };

                let next_node = self.node_pool.generate(next_state);
                // using parent (back pointer) as successor (forward pointer) instead
                current_node.set_parent(Some(next_node));
                node_stack.push(current_node);
                current_node = next_node;
                prev_state = state;
            }

            while let Some(prev_node) = node_stack.pop() {
                prev_node.set(
                    cost,
                    current_node.get(cost)
                        + octile_distance(prev_node.get(state), current_node.get(state)),
                );
                current_node = prev_node;
            }

            let new_cost = current_node.get(cost) + edge.cost;

            if new_cost < start_node.get(cost) {
                start_node.set(cost, new_cost);
                // using parent (back pointer) as successor (forward pointer) instead
                start_node.set_parent(Some(current_node));
            }
        }

        let mut path = vec![start];
        let mut node = start_node;
        while let Some(next_node) = node.get_parent() {
            path.push(next_node.get(state));
            node = next_node;
        }

        (path, start_node.get(cost))
    }
}
