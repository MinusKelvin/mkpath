#![deny(unsafe_op_in_unsafe_fn)]
//! 2D grid types and algorithms for `mkpath`.

mod bitgrid;
mod eight_connected;
mod grid;
mod grid_pool;
pub mod bucket_queue;

use enumset::EnumSetType;
use mkpath_core::traits::{Cost, EdgeId, NodePool, Successor};
use mkpath_core::{HashPool, NodeRef, NullPool};

pub use self::bitgrid::*;
pub use self::eight_connected::*;
pub use self::grid::*;
pub use self::grid_pool::*;

pub const SAFE_SQRT_2: f64 = std::f32::consts::SQRT_2 as f64;

#[derive(EnumSetType, Debug, Hash)]
pub enum Direction {
    North,
    West,
    South,
    East,
    NorthWest,
    SouthWest,
    SouthEast,
    NorthEast,
}

impl TryFrom<usize> for Direction {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Direction::North),
            1 => Ok(Direction::West),
            2 => Ok(Direction::South),
            3 => Ok(Direction::East),
            4 => Ok(Direction::NorthWest),
            5 => Ok(Direction::SouthWest),
            6 => Ok(Direction::SouthEast),
            7 => Ok(Direction::NorthEast),
            _ => Err(()),
        }
    }
}

impl Direction {
    pub fn backwards(self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::West => Direction::East,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::NorthWest => Direction::SouthEast,
            Direction::SouthWest => Direction::NorthEast,
            Direction::SouthEast => Direction::NorthWest,
            Direction::NorthEast => Direction::SouthWest,
        }
    }

    pub fn vector(self) -> (i32, i32) {
        match self {
            Direction::North => (0, -1),
            Direction::West => (-1, 0),
            Direction::South => (0, 1),
            Direction::East => (1, 0),
            Direction::NorthWest => (-1, -1),
            Direction::SouthWest => (-1, 1),
            Direction::SouthEast => (1, 1),
            Direction::NorthEast => (1, -1),
        }
    }

    pub fn orthogonal(self) -> bool {
        matches!(
            self,
            Direction::North | Direction::East | Direction::South | Direction::West
        )
    }
}

pub struct GridEdge<'a> {
    pub successor: NodeRef<'a>,
    pub cost: f64,
    pub direction: Direction,
}

impl<'a> Successor<'a> for GridEdge<'a> {
    fn successor(&self) -> NodeRef<'a> {
        self.successor
    }
}

impl Cost for GridEdge<'_> {
    fn cost(&self) -> f64 {
        self.cost
    }
}

impl EdgeId for GridEdge<'_> {
    fn edge_id(&self) -> usize {
        self.direction as usize
    }
}

/// Trait for `NodePool`s which work on grid maps.
///
/// The purpose of this trait is to allow expansion policies to skip potential bounds checks when
/// it is known that the node being generated is in-bounds of the grid being searched.
pub trait GridNodePool: NodePool {
    fn width(&self) -> i32;
    fn height(&self) -> i32;

    /// Generates a node without bounds checking.
    ///
    /// # Safety
    /// The state must be in bounds. Specifically:
    /// - `state.0` in `0..w` where `w = self.width()`
    /// - `state.1` is in `0..h` where `h = self.height()`
    ///
    /// If, for whatever reason, the values of `self.width()` and `self.height()` change, then for
    /// the purposes of the above contract, `w` and `h` and the largest values which have ever been
    /// returned by those methods.
    unsafe fn generate_unchecked(&self, state: (i32, i32)) -> NodeRef;
}

impl GridNodePool for NullPool<(i32, i32)> {
    fn width(&self) -> i32 {
        i32::MAX
    }

    fn height(&self) -> i32 {
        i32::MAX
    }

    unsafe fn generate_unchecked(&self, state: (i32, i32)) -> NodeRef {
        self.generate(state)
    }
}

impl GridNodePool for HashPool<(i32, i32)> {
    fn width(&self) -> i32 {
        i32::MAX
    }

    fn height(&self) -> i32 {
        i32::MAX
    }

    unsafe fn generate_unchecked(&self, state: (i32, i32)) -> NodeRef {
        self.generate(state)
    }
}
