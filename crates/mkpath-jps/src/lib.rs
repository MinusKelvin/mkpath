use expander::GenericJpsExpander;
use mkpath_core::NodeRef;
use mkpath_grid::{BitGrid, Direction, GridStateMapper};
use offline_jpl::OfflineJpl;
use online_jpl::OnlineJpl;

mod expander;
mod offline_jpl;
mod online_jpl;

pub use self::offline_jpl::JumpDistDatabase;

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

pub struct JpsExpander<'a, P>(GenericJpsExpander<'a, OnlineJpl<'a>, P>);

impl<'a, P: GridStateMapper> JpsExpander<'a, P> {
    pub fn new(map: &'a JpsGrid, node_pool: &'a P, target: (i32, i32)) -> Self {
        JpsExpander(GenericJpsExpander::new(
            OnlineJpl::new(map, target),
            node_pool,
        ))
    }

    pub fn expand(&mut self, node: NodeRef<'a>, edges: &mut Vec<(NodeRef<'a>, f64)>) {
        self.0.expand(node, edges)
    }
}

pub struct JpsPlusExpander<'a, P>(GenericJpsExpander<'a, OfflineJpl<'a>, P>);

impl<'a, P: GridStateMapper> JpsPlusExpander<'a, P> {
    pub fn new(jp_db: &'a JumpDistDatabase, node_pool: &'a P, target: (i32, i32)) -> Self {
        JpsPlusExpander(GenericJpsExpander::new(
            OfflineJpl::new(jp_db, target),
            node_pool,
        ))
    }

    pub fn expand(&mut self, node: NodeRef<'a>, edges: &mut Vec<(NodeRef<'a>, f64)>) {
        self.0.expand(node, edges)
    }
}

fn reached_direction(from: (i32, i32), to: (i32, i32)) -> Option<Direction> {
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

fn signed_distance<const D: i32>(from: i32, to: i32) -> i32 {
    (to - from) * D
}

trait JumpPointLocator {
    fn map(&self) -> &BitGrid;

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
        found: &mut impl FnMut((i32, i32), f64),
        x: i32,
        y: i32,
        cost: f64,
        all_1s: i32,
    ) -> i32;

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
        found: &mut impl FnMut((i32, i32), f64),
        x: i32,
        y: i32,
        cost: f64,
        all_1s: i32,
    ) -> i32;

    /// Jumps diagonally.
    ///
    /// Preconditions:
    /// - `x`, `y` are in-bounds of `map`.
    /// - `DX`, `DY` are -1 or 1.
    /// - `x+DX`, `y+DY` is traversable.
    unsafe fn jump_diag<const DX: i32, const DY: i32>(
        &self,
        found: &mut impl FnMut((i32, i32), f64),
        x: i32,
        y: i32,
        x_all_1s: i32,
        y_all_1s: i32,
    );
}
