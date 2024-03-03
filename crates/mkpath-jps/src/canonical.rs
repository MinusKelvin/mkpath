use std::f64::consts::SQRT_2;

use mkpath_core::NodeRef;
use mkpath_grid::{BitGrid, GridStateMapper};

use crate::expander::CanonicalExpander;
use crate::JumpPointLocator;

pub struct CanonicalGridExpander<'a, P>(CanonicalExpander<'a, JumplessJpl<'a>, P>);

impl<'a, P: GridStateMapper> CanonicalGridExpander<'a, P> {
    pub fn new(map: &'a BitGrid, node_pool: &'a P) -> Self {
        CanonicalGridExpander(CanonicalExpander::new(JumplessJpl(map), node_pool))
    }

    pub fn expand(&mut self, node: NodeRef<'a>, edges: &mut Vec<(NodeRef<'a>, f64)>) {
        self.0.expand(node, edges)
    }
}

struct JumplessJpl<'a>(&'a BitGrid);

impl JumpPointLocator for JumplessJpl<'_> {
    fn map(&self) -> &BitGrid {
        &self.0
    }

    unsafe fn jump_x<const DX: i32, const DY: i32>(
        &self,
        found: &mut impl FnMut((i32, i32), f64),
        x: i32,
        y: i32,
        cost: f64,
        _all_1s: i32,
    ) -> i32 {
        found((x + DX, y), cost + 1.0);
        0
    }

    unsafe fn jump_y<const DX: i32, const DY: i32>(
        &self,
        found: &mut impl FnMut((i32, i32), f64),
        x: i32,
        y: i32,
        cost: f64,
        _all_1s: i32,
    ) -> i32 {
        found((x, y + DY), cost + 1.0);
        0
    }

    unsafe fn jump_diag<const DX: i32, const DY: i32>(
        &self,
        found: &mut impl FnMut((i32, i32), f64),
        x: i32,
        y: i32,
        _x_all_1s: i32,
        _y_all_1s: i32,
    ) {
        found((x + DX, y + DY), SQRT_2);
    }
}
