use std::f64::consts::SQRT_2;

use crate::node::NodeRef;

use super::{BitGrid, GridPool};

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

pub struct JpsExpander<'a> {
    map: &'a JpsGrid,
    node_pool: &'a GridPool,
    target: (i32, i32),
}

enum Direction {
    North,
    West,
    South,
    East,
    NorthWest,
    SouthWest,
    SouthEast,
    NorthEast,
}

impl<'a> JpsExpander<'a> {
    pub fn new(map: &'a JpsGrid, node_pool: &'a GridPool, target: (i32, i32)) -> Self {
        assert!(
            node_pool.width() >= map.map.width(),
            "node pool must be wide enough for the map"
        );
        assert!(
            node_pool.height() >= map.map.height(),
            "node pool must be tall enough for the map"
        );

        JpsExpander {
            map,
            node_pool,
            target,
        }
    }

    pub fn expand(&mut self, node: NodeRef, edges: &mut Vec<(NodeRef<'a>, f64)>) {
        let (x, y) = node.get(self.node_pool.state_member());
        assert!(
            self.map.map.get(x, y),
            "attempt to expand node at untraversable location"
        );

        let dir = node.get_parent().and_then(|parent| {
            let (px, py) = parent.get(self.node_pool.state_member());
            let dx = x - px;
            let dy = y - py;
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
            } else {
                if dx < 0 {
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
        });

        unsafe {
            let nw = self.map.map.get_unchecked(x - 1, y - 1);
            let n = self.map.map.get_unchecked(x, y - 1);
            let ne = self.map.map.get_unchecked(x + 1, y - 1);
            let w = self.map.map.get_unchecked(x - 1, y);
            let e = self.map.map.get_unchecked(x + 1, y);
            let sw = self.map.map.get_unchecked(x - 1, y + 1);
            let s = self.map.map.get_unchecked(x, y + 1);
            let se = self.map.map.get_unchecked(x + 1, y + 1);

            match dir {
                Some(Direction::North) => {
                    if n {
                        self.jump_north(edges, x, y, 0.0);
                    }
                    if !sw && w {
                        self.jump_west(edges, x, y, 0.0);
                        if n && nw {
                            self.jump_northwest(edges, x, y);
                        }
                    }
                    if !se && e {
                        self.jump_east(edges, x, y, 0.0);
                        if n && ne {
                            self.jump_northeast(edges, x, y);
                        }
                    }
                }
                Some(Direction::West) => {
                    if w {
                        self.jump_west(edges, x, y, 0.0);
                    }
                    if !ne && n {
                        self.jump_north(edges, x, y, 0.0);
                        if w && nw {
                            self.jump_northwest(edges, x, y);
                        }
                    }
                    if !se && s {
                        self.jump_south(edges, x, y, 0.0);
                        if w && sw {
                            self.jump_southwest(edges, x, y);
                        }
                    }
                }
                Some(Direction::South) => {
                    if s {
                        self.jump_south(edges, x, y, 0.0);
                    }
                    if !nw && w {
                        self.jump_west(edges, x, y, 0.0);
                        if s && sw {
                            self.jump_southwest(edges, x, y);
                        }
                    }
                    if !ne && e {
                        self.jump_east(edges, x, y, 0.0);
                        if s && se {
                            self.jump_southeast(edges, x, y);
                        }
                    }
                }
                Some(Direction::East) => {
                    if e {
                        self.jump_east(edges, x, y, 0.0);
                    }
                    if !nw && n {
                        self.jump_north(edges, x, y, 0.0);
                        if e && ne {
                            self.jump_northeast(edges, x, y);
                        }
                    }
                    if !sw && s {
                        self.jump_south(edges, x, y, 0.0);
                        if e && se {
                            self.jump_southeast(edges, x, y);
                        }
                    }
                }
                Some(Direction::NorthWest) => {
                    if n {
                        self.jump_north(edges, x, y, 0.0);
                    }
                    if w {
                        self.jump_west(edges, x, y, 0.0);
                    }
                    if n && w && nw {
                        self.jump_northwest(edges, x, y);
                    }
                }
                Some(Direction::SouthWest) => {
                    if s {
                        self.jump_south(edges, x, y, 0.0);
                    }
                    if w {
                        self.jump_west(edges, x, y, 0.0);
                    }
                    if s && w && sw {
                        self.jump_southwest(edges, x, y);
                    }
                }
                Some(Direction::SouthEast) => {
                    if s {
                        self.jump_south(edges, x, y, 0.0);
                    }
                    if e {
                        self.jump_east(edges, x, y, 0.0);
                    }
                    if s && e && se {
                        self.jump_southeast(edges, x, y);
                    }
                }
                Some(Direction::NorthEast) => {
                    if n {
                        self.jump_north(edges, x, y, 0.0);
                    }
                    if e {
                        self.jump_east(edges, x, y, 0.0);
                    }
                    if n && e && ne {
                        self.jump_northeast(edges, x, y);
                    }
                }
                None => {
                    if n {
                        self.jump_north(edges, x, y, 0.0);
                    }
                    if w {
                        self.jump_west(edges, x, y, 0.0);
                    }
                    if s {
                        self.jump_south(edges, x, y, 0.0);
                    }
                    if e {
                        self.jump_east(edges, x, y, 0.0);
                    }
                    if n && w && nw {
                        self.jump_northwest(edges, x, y);
                    }
                    if s && w && sw {
                        self.jump_southwest(edges, x, y);
                    }
                    if s && e && se {
                        self.jump_southeast(edges, x, y);
                    }
                    if n && e && ne {
                        self.jump_northeast(edges, x, y);
                    }
                }
            }
        }
    }

    unsafe fn jump_north(&self, edges: &mut Vec<(NodeRef<'a>, f64)>, x: i32, y: i32, cost: f64) {
        let (mut new_y, mut successor) = jump_left(&self.map.tmap, y, x);
        if x == self.target.0 && y > self.target.1 && new_y < self.target.1 {
            successor = true;
            new_y = self.target.1;
        }
        if successor {
            edges.push((
                self.node_pool.generate_unchecked(x, new_y),
                cost + (y - new_y) as f64,
            ));
        }
    }

    unsafe fn jump_west(&self, edges: &mut Vec<(NodeRef<'a>, f64)>, x: i32, y: i32, cost: f64) {
        let (mut new_x, mut successor) = jump_left(&self.map.map, x, y);
        if y == self.target.1 && x > self.target.0 && new_x < self.target.0 {
            successor = true;
            new_x = self.target.0;
        }
        if successor {
            edges.push((
                self.node_pool.generate_unchecked(new_x, y),
                cost + (x - new_x) as f64,
            ));
        }
    }

    unsafe fn jump_south(&self, edges: &mut Vec<(NodeRef<'a>, f64)>, x: i32, y: i32, cost: f64) {
        let (mut new_y, mut successor) = jump_right(&self.map.tmap, y, x);
        if x == self.target.0 && y < self.target.1 && new_y > self.target.1 {
            successor = true;
            new_y = self.target.1;
        }
        if successor {
            edges.push((
                self.node_pool.generate_unchecked(x, new_y),
                cost + (new_y - y) as f64,
            ));
        }
    }

    unsafe fn jump_east(&self, edges: &mut Vec<(NodeRef<'a>, f64)>, x: i32, y: i32, cost: f64) {
        let (mut new_x, mut successor) = jump_right(&self.map.map, x, y);
        if y == self.target.1 && x < self.target.0 && new_x > self.target.0 {
            successor = true;
            new_x = self.target.0;
        }
        if successor {
            edges.push((
                self.node_pool.generate_unchecked(new_x, y),
                cost + (new_x - x) as f64,
            ));
        }
    }

    unsafe fn jump_northwest(
        &self,
        edges: &mut Vec<(NodeRef<'a>, f64)>,
        x: i32,
        y: i32,
    ) {
        self.jump_diag::<-1, -1>(
            edges,
            x,
            y,
            |this, edges, x, y, cost| this.jump_west(edges, x, y, cost),
            |this, edges, x, y, cost| this.jump_north(edges, x, y, cost),
        )
    }

    unsafe fn jump_northeast(
        &self,
        edges: &mut Vec<(NodeRef<'a>, f64)>,
        x: i32,
        y: i32,
    ) {
        self.jump_diag::<1, -1>(
            edges,
            x,
            y,
            |this, edges, x, y, cost| this.jump_east(edges, x, y, cost),
            |this, edges, x, y, cost| this.jump_north(edges, x, y, cost),
        )
    }

    unsafe fn jump_southwest(
        &self,
        edges: &mut Vec<(NodeRef<'a>, f64)>,
        x: i32,
        y: i32,
    ) {
        self.jump_diag::<-1, 1>(
            edges,
            x,
            y,
            |this, edges, x, y, cost| this.jump_west(edges, x, y, cost),
            |this, edges, x, y, cost| this.jump_south(edges, x, y, cost),
        )
    }

    unsafe fn jump_southeast(
        &self,
        edges: &mut Vec<(NodeRef<'a>, f64)>,
        x: i32,
        y: i32,
    ) {
        self.jump_diag::<1, 1>(
            edges,
            x,
            y,
            |this, edges, x, y, cost| this.jump_east(edges, x, y, cost),
            |this, edges, x, y, cost| this.jump_south(edges, x, y, cost),
        )
    }

    unsafe fn jump_diag<const DX: i32, const DY: i32>(
        &self,
        edges: &mut Vec<(NodeRef<'a>, f64)>,
        mut x: i32,
        mut y: i32,
        jump_x: impl Fn(&Self, &mut Vec<(NodeRef<'a>, f64)>, i32, i32, f64),
        jump_y: impl Fn(&Self, &mut Vec<(NodeRef<'a>, f64)>, i32, i32, f64),
    ) {
        let mut cost = 0.0;
        loop {
            x += DX;
            y += DY;
            cost += SQRT_2;

            if (x, y) == self.target {
                edges.push((self.node_pool.generate_unchecked(x, y), cost));
                break;
            }

            let x_t = self.map.map.get_unchecked(x + DX, y);
            let y_t = self.map.map.get_unchecked(x, y + DY);
            let xy_t = self.map.map.get_unchecked(x + DX, y + DY);
            if x_t {
                jump_x(self, edges, x, y, cost);
            }
            if y_t {
                jump_y(self, edges, x, y, cost);
            }
            if !(x_t && y_t && xy_t) {
                break;
            }
        }
    }
}

unsafe fn jump_left(map: &BitGrid, mut x: i32, y: i32) -> (i32, bool) {
    loop {
        let row_above = map.get_row_west(x, y - 1);
        let row = map.get_row_west(x, y);
        let row_below = map.get_row_west(x, y + 1);

        let above_turning = !row_above >> 1 & row_above;
        let below_turning = !row_below >> 1 & row_below;
        let stops = (above_turning | below_turning | !row) & !0x7F;

        if stops != 0 {
            let dist = stops.leading_zeros() as i32;
            return (x - dist, row & (1 << 63 - dist) != 0);
        }

        x -= 56;
    }
}

unsafe fn jump_right(map: &BitGrid, mut x: i32, y: i32) -> (i32, bool) {
    loop {
        let row_above = map.get_row_east(x, y - 1);
        let row = map.get_row_east(x, y);
        let row_below = map.get_row_east(x, y + 1);

        let above_turning = !row_above << 1 & row_above;
        let below_turning = !row_below << 1 & row_below;
        let stops = (above_turning | below_turning | !row) & (1 << 57) - 1;

        if stops != 0 {
            let dist = stops.trailing_zeros() as i32;
            return (x + dist, row & 1 << dist != 0);
        }

        x += 56;
    }
}
