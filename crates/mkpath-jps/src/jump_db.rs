use mkpath_grid::{BitGrid, Direction, Grid};

pub struct JumpDatabase {
    db: Grid<[u16; 8]>,
}

impl JumpDatabase {
    #[inline(never)]
    pub fn new(map: &BitGrid) -> Self {
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

        JumpDatabase { db }
    }

    pub fn width(&self) -> i32 {
        self.db.width()
    }

    pub fn height(&self) -> i32 {
        self.db.height()
    }

    pub fn get(&self, x: i32, y: i32, dir: Direction) -> (i32, bool) {
        let _ = self.db[(x, y)];
        unsafe { self.get_unchecked(x, y, dir) }
    }

    pub unsafe fn get_unchecked(&self, x: i32, y: i32, dir: Direction) -> (i32, bool) {
        let raw = self.db.get_unchecked(x, y)[dir as usize];
        ((raw >> 1) as i32, raw & 1 != 0)
    }

    /// Finds the end of an orthogonal jump with target check.
    ///
    /// If this function returns `Some(d)`, then `d` is the distance to the next successor. The
    /// successor is either a jump point, or it is the target.
    ///
    /// # Safety
    /// The x and y coordinates must be in-bounds of the map.
    #[inline(always)]
    pub unsafe fn ortho_jump_unchecked(
        &self,
        x: i32,
        y: i32,
        dir: Direction,
        target: (i32, i32),
    ) -> Option<i32> {
        let (dist, successor) = self.get_unchecked(x, y, dir);

        match dir {
            Direction::North => {
                (x == target.0 && y > target.1 && y - dist <= target.1).then_some(y - target.1)
            }
            Direction::West => {
                (y == target.1 && x > target.0 && x - dist <= target.0).then_some(x - target.0)
            }
            Direction::South => {
                (x == target.0 && y < target.1 && y + dist >= target.1).then_some(target.1 - y)
            }
            Direction::East => {
                (y == target.1 && x < target.0 && x + dist >= target.0).then_some(target.0 - x)
            }
            _ => panic!("called orthogonal jump with diagonal direction"),
        }
        .or(successor.then_some(dist))
    }

    /// Finds the end of a diagonal jump with target check.
    ///
    /// If this function returns `Some((d, None))`, then `d` is the distance to the next successor.
    /// The successor is either a jump point, or it is the target. If this function returns
    /// `Some((d, Some((o, d2))))`, then `d` is the distance to the place where the search must
    /// turn in orthogonal direction `o` to reach the target, and it will have to go `d2` to get .
    ///
    /// # Safety
    /// The x and y coordinates must be in-bounds of the map.
    #[inline(always)]
    pub unsafe fn diagonal_jump_unchecked(
        &self,
        x: i32,
        y: i32,
        dir: Direction,
        target: (i32, i32),
    ) -> Option<(i32, Option<(Direction, i32)>)> {
        match dir {
            Direction::NorthWest => self.diagonal_jump_impl::<-1, -1>(x, y, target),
            Direction::SouthWest => self.diagonal_jump_impl::<-1, 1>(x, y, target),
            Direction::SouthEast => self.diagonal_jump_impl::<1, 1>(x, y, target),
            Direction::NorthEast => self.diagonal_jump_impl::<1, -1>(x, y, target),
            _ => panic!("called diagonal jump with orthogonal direction"),
        }
    }

    #[inline(always)]
    unsafe fn diagonal_jump_impl<const DX: i32, const DY: i32>(
        &self,
        x: i32,
        y: i32,
        target: (i32, i32),
    ) -> Option<(i32, Option<(Direction, i32)>)> {
        let (dir, dir_x, dir_y) = match (DX, DY) {
            (-1, -1) => (Direction::NorthWest, Direction::West, Direction::North),
            (1, -1) => (Direction::NorthEast, Direction::East, Direction::North),
            (1, 1) => (Direction::SouthEast, Direction::East, Direction::South),
            (-1, 1) => (Direction::SouthWest, Direction::West, Direction::South),
            _ => unreachable!(),
        };

        let (dist, successor) = self.get_unchecked(x, y, dir);

        let x_target_dist = DX * (target.0 - x);
        let y_target_dist = DY * (target.1 - y);

        if x_target_dist > 0 && x_target_dist < dist + !successor as i32 {
            // passed target on the x axis, so we need to check y axis followup jump
            if x_target_dist == y_target_dist {
                // hit target directly
                return Some((x_target_dist, None));
            }

            let turn_x = target.0;
            let turn_y = y + DY * x_target_dist;
            // Calculate the length of the post-turn jump to the target
            let remaining_dist = DY * (target.1 - turn_y);

            // If the remaining orthogonal jump is valid, then return the way we hit the target.
            // Note that the .1 component is always false. If it were true, then `successor` and
            // `x_target_dist == dist` would be true`, so the above condition would be false!
            if remaining_dist > 0 && remaining_dist <= self.get_unchecked(turn_x, turn_y, dir_y).0 {
                return Some((x_target_dist, Some((dir_y, remaining_dist))));
            }
        }

        if y_target_dist > 0 && y_target_dist < dist + !successor as i32 {
            // passed target on the y axis, so we need to check x axis followup jump
            let turn_x = x + DX * y_target_dist;
            let turn_y = target.1;
            // Calculate the length of the post-turn jump to the target
            let remaining_dist = DX * (target.0 - turn_x);

            // If the remaining orthogonal jump is valid, then return the way we hit the target.
            // Note that the .1 component is always false. If it were true, then `successor` and
            // `y_target_dist == dist` would be true`, so the above condition would be false!
            if remaining_dist > 0 && remaining_dist <= self.get_unchecked(turn_x, turn_y, dir_x).0 {
                return Some((y_target_dist, Some((dir_x, remaining_dist))));
            }
        }

        // Regular diagonal jump point case
        successor.then_some((dist, None))
    }
}
