//! 2D grid types and algorithms for `mkpath`.

mod bitgrid;
mod grid;
mod grid_pool;

pub mod eight_connected;

use mkpath_core::{HashPool, NodeMemberPointer, NodeRef, NullPool};

pub use self::bitgrid::*;
pub use self::grid::*;
pub use self::grid_pool::*;

/// Trait for specialized grid-to-node mappers.
///
/// The purpose of this trait is to allow expansion policies to skip potential bounds checks when
/// it is known that the node being generated is in-bounds of the grid being searched.
pub trait GridStateMapper {
    fn width(&self) -> i32;
    fn height(&self) -> i32;
    fn state_member(&self) -> NodeMemberPointer<(i32, i32)>;

    /// Generates a node without bounds checking.
    ///
    /// # Safety
    /// The state must be in bounds. Specifically:
    /// - `state.0` in `0..w`, where `w` is the largest prior return value of `self.width()`
    /// - `state.1` is in `0..h`, where `h` is the largest prior return value of `self.height()`
    unsafe fn generate_unchecked(&self, state: (i32, i32)) -> NodeRef;
}

impl GridStateMapper for NullPool<(i32, i32)> {
    fn width(&self) -> i32 {
        i32::MAX
    }

    fn height(&self) -> i32 {
        i32::MAX
    }

    fn state_member(&self) -> NodeMemberPointer<(i32, i32)> {
        self.state_member()
    }

    unsafe fn generate_unchecked(&self, state: (i32, i32)) -> NodeRef {
        self.generate(state)
    }
}

impl GridStateMapper for HashPool<(i32, i32)> {
    fn width(&self) -> i32 {
        i32::MAX
    }

    fn height(&self) -> i32 {
        i32::MAX
    }

    fn state_member(&self) -> NodeMemberPointer<(i32, i32)> {
        self.state_member()
    }

    unsafe fn generate_unchecked(&self, state: (i32, i32)) -> NodeRef {
        self.generate(state)
    }
}
