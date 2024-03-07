use mkpath_core::traits::{Expander, WeightedEdge};
use mkpath_core::NodeRef;
use mkpath_grid::{BitGrid, Direction, GridStateMapper, SAFE_SQRT_2};

use crate::{canonical_successors, skipped_past, JpsGrid};

/// Jump Point Search expander.
///
/// Harabor, D., & Grastien, A. (2014, May). Improving jump point search. In Proceedings of the
/// International Conference on Automated Planning and Scheduling (Vol. 24, pp. 128-135).
pub struct JpsExpander<'a, P> {
    node_pool: &'a P,
    map: &'a JpsGrid,
    target: (i32, i32),
}

impl<'a, P: GridStateMapper> JpsExpander<'a, P> {
    pub fn new(map: &'a JpsGrid, node_pool: &'a P, target: (i32, i32)) -> Self {
        JpsExpander {
            node_pool,
            map,
            target,
        }
    }

    /// Jumps horizontally.
    ///
    /// Preconditions:
    /// - `x`, `y` are in-bounds of `map`.
    /// - `DX` is -1 or 1.
    /// - `DY` is -1, 0, or 1.
    /// - `x+DX`, `y` is traversable.
    ///
    /// Returns the x coordinate at which the jump stopped (all_1s for adjacent jump).
    unsafe fn jump_x<const DX: i32, const DY: i32>(
        &self,
        edges: &mut Vec<WeightedEdge<'a>>,
        x: i32,
        y: i32,
        cost: f64,
        all_1s: i32,
    ) -> i32 {
        let (mut new_x, mut successor) = unsafe {
            match DX {
                -1 => jump_left::<DY>(&self.map.map, x, y, all_1s),
                1 => jump_right::<DY>(&self.map.map, x, y, all_1s),
                _ => unreachable!(),
            }
        };
        let all_1s = new_x;
        if y == self.target.1 && skipped_past::<DX>(x, new_x, self.target.0) {
            successor = true;
            new_x = self.target.0;
        }
        if successor {
            edges.push(WeightedEdge {
                successor: unsafe { self.node_pool.generate_unchecked((new_x, y)) },
                cost: cost + (DX * (new_x - x)) as f64,
            });
        }
        all_1s
    }

    /// Jumps vertically.
    ///
    /// Preconditions:
    /// - `x`, `y` are in-bounds of `map`.
    /// - `DY` is -1 or 1.
    /// - `DX` is -1, 0, or 1.
    /// - `x`, `y+DY` is traversable.
    ///
    /// Returns the y coordinate at which the jump stopped (all_1s for adjacent jump).
    unsafe fn jump_y<const DX: i32, const DY: i32>(
        &self,
        edges: &mut Vec<WeightedEdge<'a>>,
        x: i32,
        y: i32,
        cost: f64,
        all_1s: i32,
    ) -> i32 {
        let (mut new_y, mut successor) = unsafe {
            // The preconditions are upheld by the caller. Note that JpsGrid has the invariant
            // that tmap is the transpose of map.
            match DY {
                -1 => jump_left::<DX>(&self.map.tmap, y, x, all_1s),
                1 => jump_right::<DX>(&self.map.tmap, y, x, all_1s),
                _ => unreachable!(),
            }
        };
        let all_1s = new_y;
        if x == self.target.0 && skipped_past::<DY>(y, new_y, self.target.1) {
            // self.target.1 is strictly between y (in-bounds) and new_y (padded in-bounds),
            // so self.target.1 must be in-bounds (it cannot be padded in-bounds).
            successor = true;
            new_y = self.target.1;
        }
        if successor {
            // new_y is in-bounds by either the contract of jump_left, or by the conditions
            // of the prior if statement.
            edges.push(WeightedEdge {
                successor: unsafe { self.node_pool.generate_unchecked((x, new_y)) },
                cost: cost + (DY * (new_y - y)) as f64,
            })
        }
        all_1s
    }

    /// Jumps diagonally.
    ///
    /// Preconditions:
    /// - `x`, `y` are in-bounds of `map`.
    /// - `DX`, `DY` are -1 or 1.
    /// - `x+DX`, `y+DY` is traversable.
    unsafe fn jump_diag<const DX: i32, const DY: i32>(
        &self,
        edges: &mut Vec<WeightedEdge<'a>>,
        mut x: i32,
        mut y: i32,
        mut x_all_1s: i32,
        mut y_all_1s: i32,
    ) {
        unsafe {
            let mut cost = 0.0;
            // Invariant: x and y are in-bounds of map. x+DX, y+DY is traversable.
            loop {
                x += DX;
                y += DY;
                cost += SAFE_SQRT_2;

                if (x, y) == self.target {
                    // x, y is traversable, which implies x, y is in-bounds.
                    // Coordinates in-bounds of the map are also in-bounds of the node pool.
                    edges.push(WeightedEdge {
                        successor: self.node_pool.generate_unchecked((x, y)),
                        cost,
                    });
                    break;
                }

                // x, y are in-bounds, so these are all padded in-bounds.
                let x_t = self.map.map.get_unchecked(x + DX, y);
                let y_t = self.map.map.get_unchecked(x, y + DY);
                let xy_t = self.map.map.get_unchecked(x + DX, y + DY);
                if x_t {
                    // x + DX, y is traversable, so this upholds the preconditions.
                    x_all_1s = self.jump_x::<DX, DY>(edges, x, y, cost, x_all_1s);
                }
                if y_t {
                    // x, y + DY is traversable, so this upholds the preconditions.
                    y_all_1s = self.jump_y::<DX, DY>(edges, x, y, cost, y_all_1s);
                }
                if !(x_t && y_t && xy_t) {
                    break;
                }
                // if x+DX, y+DY is not traversable, the loop exited above, so the invariant holds.
            }
        }
    }
}

/// Locates the next leftwards (-x) jump point using block-based jumping.
///
/// Preconditions:
/// - `x`, `y` are in-bounds of `map`.
/// - `DY` is -1, 0, or 1.
///
/// Postconditions for return value `(jp_x, forced)`:
/// - if `forced`: `(jp_x, y)` is traversable; `jp_x` is in-bounds of `map`
/// - if `!forced`: `(jp_x, y)` is non-traversable; `jp_x` is padded in-bounds of `map`
/// - `jp_x` is less than `x`
#[inline(always)]
unsafe fn jump_left<const DY: i32>(map: &BitGrid, mut x: i32, y: i32, all_1s: i32) -> (i32, bool) {
    unsafe {
        // See jump_right below; the logic is the same, except with reversed bit order.
        while DY != 0 && x >= all_1s + 56 {
            let row_adj = map.get_row_left(x, y + DY);
            let row = map.get_row_left(x, y);

            let adj_turning = !row_adj >> 1 & row_adj;
            let stops = (adj_turning | !row) & !0x7F;

            if stops != 0 {
                let dist = stops.leading_zeros() as i32;
                return (x - dist, row & (1 << (63 - dist)) != 0);
            }

            x -= 56;
        }
        loop {
            let row_above = map.get_row_left(x, y - 1);
            let row = map.get_row_left(x, y);
            let row_below = map.get_row_left(x, y + 1);

            let above_turning = !row_above >> 1 & row_above;
            let below_turning = !row_below >> 1 & row_below;
            let stops = (above_turning | below_turning | !row) & !0x7F;

            if stops != 0 {
                let dist = stops.leading_zeros() as i32;
                return (x - dist, row & (1 << (63 - dist)) != 0);
            }

            x -= 56;
        }
    }
}

/// Locates the next rightwards (+x) jump point using block-based jumping.
///
/// Preconditions:
/// - `x`, `y` are in-bounds of `map`.
/// - `DY` is -1, 0, or 1.
///
/// Postconditions for return value `(jp_x, forced)`:
/// - if `forced`: `(jp_x, y)` is traversable; `jp_x` is in-bounds of `map`
/// - if `!forced`: `(jp_x, y)` is non-traversable; `jp_x` is padded in-bounds of `map`
/// - `jp_x` is greater than `x`
#[inline(always)]
unsafe fn jump_right<const DY: i32>(map: &BitGrid, mut x: i32, y: i32, all_1s: i32) -> (i32, bool) {
    unsafe {
        // This loop's logic is very similar to the following loop's logic.
        // DY == 0 disables all_1s-optimized jumps.
        // DY != 0 assumes that the -DY row is 1s as long as x < all_1s, so we don't check it.
        // This saves a get_row call and 4 bitops, about 3% on large maps.
        // We stop when the next block could contain a jump point on the -DY side, and switch to
        // normal jumping.
        while DY != 0 && x <= all_1s - 56 {
            // y is in-bounds and abs(DY) == 1, so y + DY must be padded in-bounds, as required.
            let row_adj = map.get_row_right(x, y + DY);
            let row = map.get_row_right(x, y);

            let adj_turning = !row_adj << 1 & row_adj;
            let stops = (adj_turning | !row) & ((1 << 57) - 1);

            if stops != 0 {
                let dist = stops.trailing_zeros() as i32;
                return (x + dist, row & 1 << dist != 0);
            }

            x += 56;
        }
        // Invariant: x and y are in-bounds of map.
        loop {
            // y is in-bounds, so y +- 1 must be padded in-bounds, as required.
            let row_above = map.get_row_right(x, y - 1);
            let row = map.get_row_right(x, y);
            let row_below = map.get_row_right(x, y + 1);

            // This puts a 1 where a 0 -> 1 pattern occurs, which is a jump point.
            let above_turning = !row_above << 1 & row_above;
            let below_turning = !row_below << 1 & row_below;
            let stops = (above_turning | below_turning | !row) & ((1 << 57) - 1);

            if stops != 0 {
                let dist = stops.trailing_zeros() as i32;
                // x + dist is not traversable if we hit a dead-end instead of a jump point.
                // if we hit a dead end, then x + dist could be merely padded in-bounds of map, but
                // otherwise it is in-bounds.
                return (x + dist, row & 1 << dist != 0);
            }

            // row must have 57 1-bits in a row if stops == 0, so everything from x to x+56 is
            // traversable. The padding cells cannot have been crossed, so x is still in-bounds.
            x += 56;
        }
    }
}

impl<'a, P: GridStateMapper> Expander<'a> for JpsExpander<'a, P> {
    type Edge = WeightedEdge<'a>;

    fn expand(&mut self, node: NodeRef<'a>, edges: &mut Vec<Self::Edge>) {
        let (x, y) = node.get(self.node_pool.state_member());

        let dir = node.get_parent().and_then(|parent| {
            let (px, py) = parent.get(self.node_pool.state_member());
            crate::reached_direction((px, py), (x, y))
        });

        let successors = canonical_successors(self.map.map.get_neighborhood(x, y), dir);

        unsafe {
            // All jumps have the traversability of the relevant tile checked via successor set.
            // Remaining preconditions hold trivially.

            let mut north_all_1s = y;
            let mut south_all_1s = y;
            let mut east_all_1s = x;
            let mut west_all_1s = x;
            if successors.contains(Direction::North) {
                north_all_1s = self.jump_y::<0, -1>(edges, x, y, 0.0, 0);
            }
            if successors.contains(Direction::West) {
                west_all_1s = self.jump_x::<-1, 0>(edges, x, y, 0.0, 0);
            }
            if successors.contains(Direction::South) {
                south_all_1s = self.jump_y::<0, 1>(edges, x, y, 0.0, 0);
            }
            if successors.contains(Direction::East) {
                east_all_1s = self.jump_x::<1, 0>(edges, x, y, 0.0, 0);
            }
            if successors.contains(Direction::NorthWest) {
                self.jump_diag::<-1, -1>(edges, x, y, west_all_1s, north_all_1s);
            }
            if successors.contains(Direction::SouthWest) {
                self.jump_diag::<-1, 1>(edges, x, y, west_all_1s, south_all_1s);
            }
            if successors.contains(Direction::SouthEast) {
                self.jump_diag::<1, 1>(edges, x, y, east_all_1s, south_all_1s);
            }
            if successors.contains(Direction::NorthEast) {
                self.jump_diag::<1, -1>(edges, x, y, east_all_1s, north_all_1s);
            }
        }
    }
}
