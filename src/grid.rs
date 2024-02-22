mod bitgrid;
mod bitgrid_expander;
mod grid;
mod grid_pool;
mod jps_expander;

use crate::general::HashPool;
use crate::general::NullPool;
use crate::node::NodeMemberPointer;
use crate::node::NodeRef;

pub use self::bitgrid::*;
pub use self::bitgrid_expander::*;
pub use self::grid::*;
pub use self::grid_pool::*;
pub use self::jps_expander::*;

pub unsafe trait GridStateMapper {
    fn width(&self) -> i32;
    fn height(&self) -> i32;
    fn state_member(&self) -> NodeMemberPointer<(i32, i32)>;
    unsafe fn generate_unchecked(&self, state: (i32, i32)) -> NodeRef;
}

unsafe impl GridStateMapper for NullPool<(i32, i32)> {
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

unsafe impl GridStateMapper for HashPool<(i32, i32)> {
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
