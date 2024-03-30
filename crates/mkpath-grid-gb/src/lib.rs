use std::io::{Read, Write};

use ahash::HashMap;
use enumset::EnumSet;
use mkpath_core::NodeBuilder;
use mkpath_cpd::StateIdMapper;
use mkpath_grid::{BitGrid, Direction, EightConnectedExpander, Grid, GridPool};
use mkpath_jps::{canonical_successors, JumpDatabase};

mod bb;
mod cpd;
mod first_move;
mod tiebreak;
mod tops_expander;
mod jps_bb_expander;

pub use self::bb::*;
pub use self::cpd::*;
pub use self::tops_expander::*;
pub use self::jps_bb_expander::*;

fn independent_jump_points(
    map: &BitGrid,
    jump_db: &JumpDatabase,
) -> HashMap<(i32, i32), EnumSet<Direction>> {
    use Direction::*;

    let diagonals = NorthWest | SouthWest | NorthEast | SouthEast;

    let mut jump_points = HashMap::default();
    for y in 0..map.height() {
        for x in 0..map.width() {
            if !map.get(x, y) {
                continue;
            }

            let nb = map.get_neighborhood(x, y);
            let mut jp_successors = EnumSet::empty();
            let mut jps = EnumSet::empty();

            for dir in [North, South, East, West] {
                if !nb.contains(dir.backwards()) {
                    continue;
                }
                let dirs = canonical_successors(nb, Some(dir));
                if dirs & dir != dirs {
                    jps |= dir;
                    jp_successors |= dirs;
                }
            }

            if !jps.is_empty() {
                *jump_points.entry((x, y)).or_default() |= jps;

                jp_successors &= diagonals;
                if jp_successors.contains(NorthWest) {
                    collect_diagonal_jps(&mut jump_points, &jump_db, x, y, NorthWest);
                }
                if jp_successors.contains(SouthWest) {
                    collect_diagonal_jps(&mut jump_points, &jump_db, x, y, SouthWest);
                }
                if jp_successors.contains(SouthEast) {
                    collect_diagonal_jps(&mut jump_points, &jump_db, x, y, SouthEast);
                }
                if jp_successors.contains(NorthEast) {
                    collect_diagonal_jps(&mut jump_points, &jump_db, x, y, NorthEast);
                }
            }
        }
    }

    jump_points
}

fn collect_diagonal_jps(
    jump_points: &mut HashMap<(i32, i32), EnumSet<Direction>>,
    jump_db: &JumpDatabase,
    mut x: i32,
    mut y: i32,
    dir: Direction,
) {
    let (dx, dy) = match dir {
        Direction::NorthWest => (-1, -1),
        Direction::SouthWest => (-1, 1),
        Direction::SouthEast => (1, 1),
        Direction::NorthEast => (1, -1),
        _ => unreachable!(),
    };

    while let (dist, true) = jump_db.get(x, y, dir) {
        x += dx * dist;
        y += dy * dist;
        *jump_points.entry((x, y)).or_default() |= dir;
    }
}

pub struct GridMapper {
    grid: Grid<usize>,
    array: Box<[(i32, i32)]>,
}

impl GridMapper {
    pub fn dfs_preorder(map: &BitGrid) -> Self {
        let mut grid = Grid::new(map.width(), map.height(), |_, _| usize::MAX);
        let mut array = vec![];

        let mut builder = NodeBuilder::new();
        let state = builder.add_field((-1, -1));
        let mut pool = GridPool::new(builder.build(), state, map.width(), map.height());

        for y in 0..map.height() {
            for x in 0..map.width() {
                if !map.get(x, y) || grid[(x, y)] != usize::MAX {
                    continue;
                }

                pool.reset();
                mkpath_cpd::dfs_traversal(
                    pool.generate((x, y)),
                    EightConnectedExpander::new(&map, &pool),
                    |node| {
                        if grid[node.get(state)] == usize::MAX {
                            grid[node.get(state)] = array.len();
                            array.push(node.get(state));
                            true
                        } else {
                            false
                        }
                    },
                );
            }
        }

        GridMapper {
            grid,
            array: array.into_boxed_slice(),
        }
    }

    pub fn load(from: &mut impl Read) -> std::io::Result<Self> {
        let mut bytes = [0; 4];
        from.read_exact(&mut bytes)?;
        let len = u32::from_le_bytes(bytes) as usize;

        from.read_exact(&mut bytes)?;
        let width = i32::from_le_bytes(bytes);
        from.read_exact(&mut bytes)?;
        let height = i32::from_le_bytes(bytes);

        let mut grid = Grid::new(width, height, |_, _| usize::MAX);
        let mut array = vec![(0, 0); len].into_boxed_slice();
        for id in 0..len {
            from.read_exact(&mut bytes)?;
            let x = i32::from_le_bytes(bytes);
            from.read_exact(&mut bytes)?;
            let y = i32::from_le_bytes(bytes);
            grid[(x, y)] = id;
            array[id] = (x, y);
        }

        Ok(GridMapper { grid, array })
    }

    pub fn save(&self, to: &mut impl Write) -> std::io::Result<()> {
        to.write_all(&(self.array.len() as u32).to_le_bytes())?;
        to.write_all(&self.grid.width().to_le_bytes())?;
        to.write_all(&self.grid.height().to_le_bytes())?;
        for (x, y) in self.array.iter() {
            to.write_all(&x.to_le_bytes())?;
            to.write_all(&y.to_le_bytes())?;
        }
        Ok(())
    }
}

impl StateIdMapper for GridMapper {
    type State = (i32, i32);

    fn num_ids(&self) -> usize {
        self.array.len()
    }

    fn state_to_id(&self, state: Self::State) -> usize {
        self.grid[state]
    }

    fn id_to_state(&self, id: usize) -> Self::State {
        self.array[id]
    }
}
