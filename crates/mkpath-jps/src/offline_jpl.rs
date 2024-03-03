use std::f64::consts::SQRT_2;

use mkpath_grid::{BitGrid, Grid};

use crate::{in_direction, signed_distance, skipped_past, Direction, JumpPointLocator};

pub(crate) struct OfflineJpl<'a> {
    jp_db: &'a JumpDatabase,
    target: (i32, i32),
}

pub struct JumpDatabase {
    map: BitGrid,
    db: Grid<[u16; 8]>,
}

impl JumpDatabase {
    #[inline(never)]
    pub fn new(map: BitGrid) -> Self {
        use Direction::*;

        assert!(
            map.width() <= 1 << 15,
            "map cannot be wider than 32768 tiles"
        );
        assert!(
            map.height() <= 1 << 15,
            "map cannot be taller than 32768 tiles"
        );

        let mut db = Grid::new(map.width(), map.height(), |_, _| [0; 8]);

        for y in 0..map.height() {
            for x in 0..map.width() {
                let nb = map.get_neighborhood(x, y);

                // West
                if nb & (West | NorthWest | North) == West | NorthWest
                    || nb & (West | SouthWest | South) == West | SouthWest
                {
                    // The location to the west is a jump point; distance 1, successor.
                    // DB values are encoded distance << 1 | successor
                    db[(x, y)][West as usize] = 3;
                } else if nb.contains(West) {
                    // The location to the west is not a jump point, but we can jump through it.
                    // Increase the distance by 1 and keep the successor flag.
                    db[(x, y)][West as usize] = db[(x - 1, y)][West as usize] + 2;
                } else {
                    // If we can't go west, then the jump distance is 0 and there is no successor.
                    // This is represented by db value 0, which is the default, so we don't need
                    // to do anything.
                }

                // North
                // This works basically the same as the above logic.
                if nb & (North | NorthWest | West) == North | NorthWest
                    || nb & (North | NorthEast | East) == North | NorthEast
                {
                    db[(x, y)][North as usize] = 3;
                } else if nb.contains(North) {
                    db[(x, y)][North as usize] = db[(x, y - 1)][North as usize] + 2;
                }
            }
        }

        for y in (0..map.height()).rev() {
            for x in (0..map.width()).rev() {
                let nb = map.get_neighborhood(x, y);

                // East
                if nb & (East | NorthEast | North) == East | NorthEast
                    || nb & (East | SouthEast | South) == East | SouthEast
                {
                    db[(x, y)][East as usize] = 3;
                } else if nb.contains(East) {
                    db[(x, y)][East as usize] = db[(x + 1, y)][East as usize] + 2;
                }

                // South
                if nb & (South | SouthWest | West) == South | SouthWest
                    || nb & (South | SouthEast | East) == South | SouthEast
                {
                    db[(x, y)][South as usize] = 3;
                } else if nb.contains(South) {
                    db[(x, y)][South as usize] = db[(x, y + 1)][South as usize] + 2;
                }
            }
        }

        for y in 0..map.height() {
            for x in 0..map.width() {
                let nb = map.get_neighborhood(x, y);

                // NorthWest
                if nb.is_superset(North | West | NorthWest) {
                    // We can go northwest. The northwest tile is a jump point if at least one of
                    // the north or west jumps have successors.
                    if db[(x - 1, y - 1)][West as usize] & 1 != 0
                        || db[(x - 1, y - 1)][North as usize] & 1 != 0
                    {
                        // At least one of the orthogonal jumps for the next tile has a successor;
                        // distance 1, successor.
                        db[(x, y)][NorthWest as usize] = 3;
                    } else {
                        // The location to the west is not a jump point, but we can jump through it.
                        // Increase the distance by 1 and keep the successor flag.
                        db[(x, y)][NorthWest as usize] = db[(x - 1, y - 1)][NorthWest as usize] + 2;
                    }
                } else {
                    // If we can't go northwest, then the jump distance is 0 and there is no
                    // successor. This is represented by db value 0, which is the default, so we
                    // don't need to do anything.
                }

                // NorthEast
                // This works basically the same as the above logic.
                if nb.is_superset(North | East | NorthEast) {
                    if db[(x + 1, y - 1)][East as usize] & 1 != 0
                        || db[(x + 1, y - 1)][North as usize] & 1 != 0
                    {
                        db[(x, y)][NorthEast as usize] = 3;
                    } else {
                        db[(x, y)][NorthEast as usize] = db[(x + 1, y - 1)][NorthEast as usize] + 2;
                    }
                }
            }
        }

        for y in (0..map.height()).rev() {
            for x in (0..map.width()).rev() {
                let nb = map.get_neighborhood(x, y);

                // SouthWest
                if nb.is_superset(South | West | SouthWest) {
                    if db[(x - 1, y + 1)][West as usize] & 1 != 0
                        || db[(x - 1, y + 1)][South as usize] & 1 != 0
                    {
                        db[(x, y)][SouthWest as usize] = 3;
                    } else {
                        db[(x, y)][SouthWest as usize] = db[(x - 1, y + 1)][SouthWest as usize] + 2;
                    }
                }

                // SouthEast
                if nb.is_superset(South | East | SouthEast) {
                    if db[(x + 1, y + 1)][East as usize] & 1 != 0
                        || db[(x + 1, y + 1)][South as usize] & 1 != 0
                    {
                        db[(x, y)][SouthEast as usize] = 3;
                    } else {
                        db[(x, y)][SouthEast as usize] = db[(x + 1, y + 1)][SouthEast as usize] + 2;
                    }
                }
            }
        }

        JumpDatabase { map, db }
    }

    pub fn get(&self, x: i32, y: i32, dir: Direction) -> (i32, bool) {
        let _ = self.db[(x, y)];
        unsafe { self.get_unchecked(x, y, dir) }
    }

    pub unsafe fn get_unchecked(&self, x: i32, y: i32, dir: Direction) -> (i32, bool) {
        let raw = self.db.get_unchecked(x, y)[dir as usize];
        ((raw >> 1) as i32, raw & 1 != 0)
    }
}

impl<'a> OfflineJpl<'a> {
    pub fn new(jp_db: &'a JumpDatabase, target: (i32, i32)) -> Self {
        OfflineJpl { jp_db, target }
    }
}

impl<'a> JumpPointLocator for OfflineJpl<'a> {
    fn map(&self) -> &BitGrid {
        &self.jp_db.map
    }

    unsafe fn jump_x<const DX: i32, const DY: i32>(
        &self,
        found: &mut impl FnMut((i32, i32), f64),
        x: i32,
        y: i32,
        cost: f64,
        _all_1s: i32,
    ) -> i32 {
        let (mut new_x, mut successor) = unsafe {
            match DX {
                -1 => self.jp_db.get_unchecked(x, y, Direction::West),
                1 => self.jp_db.get_unchecked(x, y, Direction::East),
                _ => unreachable!(),
            }
        };
        new_x = x + DX * new_x;
        let all_1s = new_x;
        if y == self.target.1 && skipped_past::<DX>(x, new_x + DX, self.target.0) {
            successor = true;
            new_x = self.target.0;
        }
        if successor {
            found((new_x, y), cost + (DX * (new_x - x)) as f64);
        }
        all_1s
    }

    unsafe fn jump_y<const DX: i32, const DY: i32>(
        &self,
        found: &mut impl FnMut((i32, i32), f64),
        x: i32,
        y: i32,
        cost: f64,
        _all_1s: i32,
    ) -> i32 {
        let (mut new_y, mut successor) = unsafe {
            // The preconditions are upheld by the caller.
            match DY {
                -1 => self.jp_db.get_unchecked(x, y, Direction::North),
                1 => self.jp_db.get_unchecked(x, y, Direction::South),
                _ => unreachable!(),
            }
        };
        new_y = y + DY * new_y;
        let all_1s = new_y;
        if x == self.target.0 && skipped_past::<DY>(y, new_y + DY, self.target.1) {
            // self.target.1 is strictly between y (in-bounds) and new_y (padded in-bounds),
            // so self.target.1 must be in-bounds (it cannot be padded in-bounds).
            successor = true;
            new_y = self.target.1;
        }
        if successor {
            // new_y is in-bounds by either the contract of jump_left, or by the conditions
            // of the prior if statement.
            found((x, new_y), cost + (DY * (new_y - y)) as f64)
        }
        all_1s
    }

    unsafe fn jump_diag<const DX: i32, const DY: i32>(
        &self,
        found: &mut impl FnMut((i32, i32), f64),
        mut x: i32,
        mut y: i32,
        _x_all_1s: i32,
        _y_all_1s: i32,
    ) {
        let dir = match (DX, DY) {
            (-1, -1) => Direction::NorthWest,
            (-1, 1) => Direction::SouthWest,
            (1, -1) => Direction::NorthEast,
            (1, 1) => Direction::SouthEast,
            _ => unreachable!(),
        };
        let mut cost = 0.0;

        loop {
            let (dist, successor) = self.jp_db.get_unchecked(x, y, dir);
            let new_x = x + DX * dist;
            let new_y = y + DY * dist;

            let extended_x = match successor {
                true => new_x,
                false => new_x + DX,
            };
            let extended_y = match successor {
                true => new_y,
                false => new_y + DY,
            };

            if skipped_past::<DX>(x, extended_x, self.target.0) {
                let dist = signed_distance::<DX>(x, self.target.0);
                let cost = cost + dist as f64 * SQRT_2;
                let new_x = self.target.0;
                let new_y = y + DY * dist;
                if (new_x, new_y) == self.target {
                    found((new_x, new_y), cost);
                    break;
                }
                if in_direction::<DY>(new_y, self.target.1) {
                    self.jump_y::<DX, DY>(found, new_x, new_y, cost, 0);
                }
            }

            if skipped_past::<DY>(y, extended_y, self.target.1) {
                let dist = signed_distance::<DY>(y, self.target.1);
                let cost = cost + dist as f64 * SQRT_2;
                let new_x = x + DX * dist;
                let new_y = self.target.1;
                if (new_x, new_y) == self.target {
                    found((new_x, new_y), cost);
                    break;
                }
                if in_direction::<DX>(new_x, self.target.0) {
                    self.jump_x::<DX, DY>(found, new_x, new_y, cost, 0);
                }
            }

            x = new_x;
            y = new_y;
            cost += dist as f64 * SQRT_2;

            if (x, y) == self.target {
                found((x, y), cost);
                break;
            }

            if !successor {
                break;
            }

            self.jump_x::<DX, DY>(found, x, y, cost, 0);
            self.jump_y::<DX, DY>(found, x, y, cost, 0);
        }
    }
}
