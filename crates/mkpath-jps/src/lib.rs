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
    const N: u8 = 1 << Direction::North as usize;
    const W: u8 = 1 << Direction::West as usize;
    const S: u8 = 1 << Direction::South as usize;
    const E: u8 = 1 << Direction::East as usize;
    const NW: u8 = 1 << Direction::NorthWest as usize;
    const SW: u8 = 1 << Direction::SouthWest as usize;
    const SE: u8 = 1 << Direction::SouthEast as usize;
    const NE: u8 = 1 << Direction::NorthEast as usize;

    const fn ortho_successors(f: u8, fl: u8, l: u8, bl: u8, fr: u8, r: u8, br: u8) -> [u8; 256] {
        let mut result = [0; 256];
        let mut nb = 0;
        while nb < 256 {
            if nb as u8 & f != 0 {
                result[nb] |= f;
            }
            if nb as u8 & (bl | l) == l {
                result[nb] |= l;
                if nb as u8 & (f | fl) == f | fl {
                    result[nb] |= fl;
                }
            }
            if nb as u8 & (br | r) == r {
                result[nb] |= r;
                if nb as u8 & (f | fr) == f | fr {
                    result[nb] |= fr;
                }
            }
            nb += 1;
        }
        result
    }

    const fn diagonal_successors(f: u8, l: u8, r: u8) -> [u8; 256] {
        let mut result = [0; 256];
        let mut nb = 0;
        while nb < 256 {
            if nb as u8 & l != 0 {
                result[nb] |= l;
            }
            if nb as u8 & r != 0 {
                result[nb] |= r;
            }
            if nb as u8 & (l | r | f) == l | r | f {
                result[nb] |= f;
            }
            nb += 1;
        }
        result
    }

    static SUCCESSORS: [[u8; 256]; 9] = [
        ortho_successors(N, NW, W, SW, NE, E, SE),
        ortho_successors(W, SW, S, SE, NW, N, NE),
        ortho_successors(S, SE, E, NE, SW, W, NW),
        ortho_successors(E, NE, N, NW, SE, S, SW),
        diagonal_successors(NW, N, W),
        diagonal_successors(SW, S, W),
        diagonal_successors(SE, S, E),
        diagonal_successors(NE, N, E),
        {
            let mut result = [0; 256];
            let mut nb = 0;
            while nb < 256 {
                if nb as u8 & N != 0 {
                    result[nb] |= N;
                }
                if nb as u8 & W != 0 {
                    result[nb] |= W;
                }
                if nb as u8 & S != 0 {
                    result[nb] |= S;
                }
                if nb as u8 & E != 0 {
                    result[nb] |= E;
                }
                if nb as u8 & (N | W | NW) == N | W | NW {
                    result[nb] |= NW;
                }
                if nb as u8 & (S | W | SW) == S | W | SW {
                    result[nb] |= SW;
                }
                if nb as u8 & (S | E | SE) == S | E | SE {
                    result[nb] |= SE;
                }
                if nb as u8 & (N | E | NE) == N | E | NE {
                    result[nb] |= NE;
                }
                nb += 1;
            }
            result
        },
    ];

    EnumSet::from_u8(SUCCESSORS[going.map_or(8, |d| d as usize)][nb.as_usize()])
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
