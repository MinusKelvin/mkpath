use ahash::HashMap;
use enumset::EnumSet;
use mkpath_grid::Direction;
use mkpath_jps::{canonical_successors, JumpDatabase};

mod bb;
mod cpd;
mod first_move;
mod jps_bb_expander;
mod mapper;
mod tiebreak;
mod tops_expander;

pub use self::bb::*;
pub use self::cpd::*;
pub use self::jps_bb_expander::*;
pub use self::tops_expander::*;

fn independent_jump_points(jump_db: &JumpDatabase) -> HashMap<(i32, i32), EnumSet<Direction>> {
    use Direction::*;

    let map = jump_db.map();

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
