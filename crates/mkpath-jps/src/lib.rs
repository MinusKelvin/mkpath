use enumset::EnumSet;
use mkpath_grid::{BitGrid, Direction};

mod canonical;
mod jps;
mod jps_plus;
mod jump_db;

pub use self::canonical::*;
pub use self::jps::*;
pub use self::jps_plus::*;
pub use self::jump_db::*;

pub struct JpsGrid {
    map: BitGrid,
    tmap: BitGrid,
}

impl From<BitGrid> for JpsGrid {
    fn from(map: BitGrid) -> Self {
        let mut tmap = BitGrid::new(map.height(), map.width());
        for x in 0..tmap.width() {
            for y in 0..tmap.height() {
                tmap.set(x, y, map.get(y, x));
            }
        }
        JpsGrid { map, tmap }
    }
}

pub fn reached_direction(from: (i32, i32), to: (i32, i32)) -> Option<Direction> {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    if dx.abs() > dy.abs() {
        if dx < 0 {
            Some(Direction::West)
        } else {
            Some(Direction::East)
        }
    } else if dy.abs() > dx.abs() {
        if dy < 0 {
            Some(Direction::North)
        } else {
            Some(Direction::South)
        }
    } else if dx < 0 {
        if dy < 0 {
            Some(Direction::NorthWest)
        } else {
            Some(Direction::SouthWest)
        }
    } else if dx > 0 {
        if dy < 0 {
            Some(Direction::NorthEast)
        } else {
            Some(Direction::SouthEast)
        }
    } else {
        None
    }
}

pub fn canonical_successors(
    nb: EnumSet<Direction>,
    going: Option<Direction>,
) -> EnumSet<Direction> {
    let mut successors = EnumSet::empty();
    use Direction::*;
    match going {
        Some(North) => {
            if nb.contains(North) {
                successors |= North;
            }
            if nb & (SouthWest | West) == West {
                successors |= West;
                if nb.is_superset(North | NorthWest) {
                    successors |= NorthWest;
                }
            }
            if nb & (SouthEast | East) == East {
                successors |= East;
                if nb.is_superset(North | NorthEast) {
                    successors |= NorthEast;
                }
            }
        }
        Some(West) => {
            if nb.contains(West) {
                successors |= West;
            }
            if nb & (NorthEast | North) == North {
                successors |= North;
                if nb.is_superset(West | NorthWest) {
                    successors |= NorthWest;
                }
            }
            if nb & (SouthEast | South) == South {
                successors |= South;
                if nb.is_superset(West | SouthWest) {
                    successors |= SouthWest;
                }
            }
        }
        Some(South) => {
            if nb.contains(South) {
                successors |= South;
            }
            if nb & (NorthWest | West) == West {
                successors |= West;
                if nb.is_superset(South | SouthWest) {
                    successors |= SouthWest;
                }
            }
            if nb & (NorthEast | East) == East {
                successors |= East;
                if nb.is_superset(South | SouthEast) {
                    successors |= SouthEast;
                }
            }
        }
        Some(East) => {
            if nb.contains(East) {
                successors |= East;
            }
            if nb & (NorthWest | North) == North {
                successors |= North;
                if nb.is_superset(East | NorthEast) {
                    successors |= NorthEast;
                }
            }
            if nb & (SouthWest | South) == South {
                successors |= South;
                if nb.is_superset(East | SouthEast) {
                    successors |= SouthEast;
                }
            }
        }
        Some(NorthWest) => {
            if nb.contains(North) {
                successors |= North;
            }
            if nb.contains(West) {
                successors |= West;
            }
            if nb.is_superset(North | West | NorthWest) {
                successors |= NorthWest;
            }
        }
        Some(SouthWest) => {
            if nb.contains(South) {
                successors |= South;
            }
            if nb.contains(West) {
                successors |= West;
            }
            if nb.is_superset(South | West | SouthWest) {
                successors |= SouthWest;
            }
        }
        Some(SouthEast) => {
            if nb.contains(South) {
                successors |= South;
            }
            if nb.contains(East) {
                successors |= East;
            }
            if nb.is_superset(South | East | SouthEast) {
                successors |= SouthEast;
            }
        }
        Some(NorthEast) => {
            if nb.contains(North) {
                successors |= North;
            }
            if nb.contains(East) {
                successors |= East;
            }
            if nb.is_superset(North | East | NorthEast) {
                successors |= NorthEast;
            }
        }
        None => {
            if nb.contains(North) {
                successors |= North;
            }
            if nb.contains(West) {
                successors |= West;
            }
            if nb.contains(South) {
                successors |= South;
            }
            if nb.contains(East) {
                successors |= East;
            }
            if nb.is_superset(North | West | NorthWest) {
                successors |= NorthWest;
            }
            if nb.is_superset(South | West | SouthWest) {
                successors |= SouthWest;
            }
            if nb.is_superset(South | East | SouthEast) {
                successors |= SouthEast;
            }
            if nb.is_superset(North | East | NorthEast) {
                successors |= NorthEast;
            }
        }
    }
    successors
}

fn skipped_past<const D: i32>(start: i32, end: i32, target: i32) -> bool {
    in_direction::<D>(start, target) && in_direction::<D>(target, end)
}

fn in_direction<const D: i32>(from: i32, to: i32) -> bool {
    match D {
        -1 => to < from,
        1 => from < to,
        _ => unreachable!(),
    }
}
