//! # `mkpath-grid-gb`
//!
//! Goal Bounding techniques for grid pathfinding.
//!
//! This crate implements algorithms from *Regarding goal bounding and jump point search*
//! (Hu et al, 2021) which utilize partial goal bounding data:
//! - JPS+BB+ (JPS+ augmented with geometric containers for move pruning)
//! - TOPS (JPS+ augmented with first-move data)
//! - Topping+ (Path extraction from first-move data)
//!
//! todo: add variants using full goal bounding data:
//! JPS+BB (Rabin & Sturtevant, 2016), Topping (Salvetti et al, 2018)
//!
//! ## References
//!
//! - Hu, Y., Harabor, D., Qin, L., & Yin, Q. (2021). Regarding goal bounding and jump point search. Journal of Artificial Intelligence Research, 70, 631-681.
//! - Rabin, S., & Sturtevant, N. (2016, February). Combining bounding boxes and JPS to prune grid pathfinding. In Proceedings of the AAAI Conference on Artificial Intelligence (Vol. 30, No. 1).
//! - Salvetti, M., Botea, A., Gerevini, A., Harabor, D., & Saetti, A. (2018, June). Two-oracle optimal path planning on grid maps. In Proceedings of the International Conference on Automated Planning and Scheduling (Vol. 28, pp. 227-231).

use std::sync::Mutex;

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
mod topping_plus;
mod tops_expander;

pub use self::bb::*;
pub use self::cpd::*;
pub use self::jps_bb_expander::*;
pub use self::topping_plus::*;
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

fn parallel_for<I, T>(
    iter: impl Iterator<Item = T> + Send,
    init: impl Fn() -> I + Sync,
    each: impl Fn(&mut I, T) -> std::io::Result<()> + Sync,
) -> std::io::Result<()> {
    let iter = Mutex::new(iter);
    std::thread::scope(|s| {
        let mut handles = vec![];
        for _ in 0..num_cpus::get() {
            handles.push(s.spawn(|| {
                let mut context = init();
                loop {
                    let mut guard = iter.lock().unwrap();
                    let Some(item) = guard.next() else {
                        return Ok(());
                    };
                    drop(guard);
                    each(&mut context, item)?;
                }
            }));
        }
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    })
}
