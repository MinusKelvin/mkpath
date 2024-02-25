use std::f64::consts::SQRT_2;

use mkpath_core::NodeRef;

use crate::{BitGrid, GridStateMapper};

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

pub struct JpsExpander<'a, P> {
    map: &'a JpsGrid,
    node_pool: &'a P,
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

impl<'a, P: GridStateMapper> JpsExpander<'a, P> {
    pub fn new(map: &'a JpsGrid, node_pool: &'a P, target: (i32, i32)) -> Self {
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
                    let mut north_verif = y;
                    if n {
                        north_verif = self.jump_y::<0, -1>(edges, x, y, 0.0, 0);
                    }
                    if !sw && w {
                        let west_verif = self.jump_x::<-1, 0>(edges, x, y, 0.0, 0);
                        if n && nw {
                            self.jump_diag::<-1, -1>(edges, x, y, west_verif, north_verif);
                        }
                    }
                    if !se && e {
                        let east_verif = self.jump_x::<1, 0>(edges, x, y, 0.0, 0);
                        if n && ne {
                            self.jump_diag::<1, -1>(edges, x, y, east_verif, north_verif);
                        }
                    }
                }
                Some(Direction::West) => {
                    let mut west_verif = x;
                    if w {
                        west_verif = self.jump_x::<-1, 0>(edges, x, y, 0.0, 0);
                    }
                    if !ne && n {
                        let north_verif = self.jump_y::<0, -1>(edges, x, y, 0.0, 0);
                        if w && nw {
                            self.jump_diag::<-1, -1>(edges, x, y, west_verif, north_verif);
                        }
                    }
                    if !se && s {
                        let south_verif = self.jump_y::<0, 1>(edges, x, y, 0.0, 0);
                        if w && sw {
                            self.jump_diag::<-1, 1>(edges, x, y, west_verif, south_verif);
                        }
                    }
                }
                Some(Direction::South) => {
                    let mut south_verif = y;
                    if s {
                        south_verif = self.jump_y::<0, 1>(edges, x, y, 0.0, 0);
                    }
                    if !nw && w {
                        let west_verif = self.jump_x::<-1, 0>(edges, x, y, 0.0, 0);
                        if s && sw {
                            self.jump_diag::<-1, 1>(edges, x, y, west_verif, south_verif);
                        }
                    }
                    if !ne && e {
                        let east_verif = self.jump_x::<1, 0>(edges, x, y, 0.0, 0);
                        if s && se {
                            self.jump_diag::<1, 1>(edges, x, y, east_verif, south_verif);
                        }
                    }
                }
                Some(Direction::East) => {
                    let mut east_verif = x;
                    if e {
                        east_verif = self.jump_x::<1, 0>(edges, x, y, 0.0, 0);
                    }
                    if !nw && n {
                        let north_verif = self.jump_y::<0, -1>(edges, x, y, 0.0, 0);
                        if e && ne {
                            self.jump_diag::<1, -1>(edges, x, y, east_verif, north_verif);
                        }
                    }
                    if !sw && s {
                        let south_verif = self.jump_y::<0, 1>(edges, x, y, 0.0, 0);
                        if e && se {
                            self.jump_diag::<1, 1>(edges, x, y, east_verif, south_verif);
                        }
                    }
                }
                Some(Direction::NorthWest) => {
                    let mut north_verif = y;
                    let mut west_verif = x;
                    if n {
                        north_verif = self.jump_y::<0, -1>(edges, x, y, 0.0, 0);
                    }
                    if w {
                        west_verif = self.jump_x::<-1, 0>(edges, x, y, 0.0, 0);
                    }
                    if n && w && nw {
                        self.jump_diag::<-1, -1>(edges, x, y, west_verif, north_verif);
                    }
                }
                Some(Direction::SouthWest) => {
                    let mut south_verif = y;
                    let mut west_verif = x;
                    if s {
                        south_verif = self.jump_y::<0, 1>(edges, x, y, 0.0, 0);
                    }
                    if w {
                        west_verif = self.jump_x::<-1, 0>(edges, x, y, 0.0, 0);
                    }
                    if s && w && sw {
                        self.jump_diag::<-1, 1>(edges, x, y, west_verif, south_verif);
                    }
                }
                Some(Direction::SouthEast) => {
                    let mut south_verif = y;
                    let mut east_verif = x;
                    if s {
                        south_verif = self.jump_y::<0, 1>(edges, x, y, 0.0, 0);
                    }
                    if e {
                        east_verif = self.jump_x::<1, 0>(edges, x, y, 0.0, 0);
                    }
                    if s && e && se {
                        self.jump_diag::<1, 1>(edges, x, y, east_verif, south_verif);
                    }
                }
                Some(Direction::NorthEast) => {
                    let mut north_verif = y;
                    let mut east_verif = x;
                    if n {
                        north_verif = self.jump_y::<0, -1>(edges, x, y, 0.0, 0);
                    }
                    if e {
                        east_verif = self.jump_x::<1, 0>(edges, x, y, 0.0, 0);
                    }
                    if n && e && ne {
                        self.jump_diag::<1, -1>(edges, x, y, east_verif, north_verif);
                    }
                }
                None => {
                    let mut north_verif = y;
                    let mut south_verif = y;
                    let mut east_verif = x;
                    let mut west_verif = x;
                    if n {
                        north_verif = self.jump_y::<0, -1>(edges, x, y, 0.0, 0);
                    }
                    if w {
                        west_verif = self.jump_x::<-1, 0>(edges, x, y, 0.0, 0);
                    }
                    if s {
                        south_verif = self.jump_y::<0, 1>(edges, x, y, 0.0, 0);
                    }
                    if e {
                        east_verif = self.jump_x::<1, 0>(edges, x, y, 0.0, 0);
                    }
                    if n && w && nw {
                        self.jump_diag::<-1, -1>(edges, x, y, west_verif, north_verif);
                    }
                    if s && w && sw {
                        self.jump_diag::<-1, 1>(edges, x, y, west_verif, south_verif);
                    }
                    if s && e && se {
                        self.jump_diag::<1, 1>(edges, x, y, east_verif, south_verif);
                    }
                    if n && e && ne {
                        self.jump_diag::<1, -1>(edges, x, y, east_verif, north_verif);
                    }
                }
            }
        }
    }

    unsafe fn jump_x<const DX: i32, const DY: i32>(
        &self,
        edges: &mut Vec<(NodeRef<'a>, f64)>,
        x: i32,
        y: i32,
        cost: f64,
        verif: i32,
    ) -> i32 {
        if DX < 0 {
            let (mut new_x, mut successor) = jump_left_verif::<DY>(&self.map.map, x, y, verif);
            let verif = new_x;
            if y == self.target.1 && x > self.target.0 && new_x < self.target.0 {
                successor = true;
                new_x = self.target.0;
            }
            if successor {
                edges.push((
                    self.node_pool.generate_unchecked((new_x, y)),
                    cost + (x - new_x) as f64,
                ));
            }
            verif
        } else if DX > 0 {
            let (mut new_x, mut successor) = jump_right_verif::<DY>(&self.map.map, x, y, verif);
            let verif = new_x;
            if y == self.target.1 && x < self.target.0 && new_x > self.target.0 {
                successor = true;
                new_x = self.target.0;
            }
            if successor {
                edges.push((
                    self.node_pool.generate_unchecked((new_x, y)),
                    cost + (new_x - x) as f64,
                ));
            }
            verif
        } else {
            unreachable!()
        }
    }

    unsafe fn jump_y<const DX: i32, const DY: i32>(
        &self,
        edges: &mut Vec<(NodeRef<'a>, f64)>,
        x: i32,
        y: i32,
        cost: f64,
        verif: i32,
    ) -> i32 {
        if DY < 0 {
            let (mut new_y, mut successor) = jump_left_verif::<DX>(&self.map.tmap, y, x, verif);
            let verif = new_y;
            if x == self.target.0 && y > self.target.1 && new_y < self.target.1 {
                successor = true;
                new_y = self.target.1;
            }
            if successor {
                edges.push((
                    self.node_pool.generate_unchecked((x, new_y)),
                    cost + (y - new_y) as f64,
                ));
            }
            verif
        } else if DY > 0 {
            let (mut new_y, mut successor) = jump_right_verif::<DX>(&self.map.tmap, y, x, verif);
            let verif = new_y;
            if x == self.target.0 && y < self.target.1 && new_y > self.target.1 {
                successor = true;
                new_y = self.target.1;
            }
            if successor {
                edges.push((
                    self.node_pool.generate_unchecked((x, new_y)),
                    cost + (new_y - y) as f64,
                ));
            }
            verif
        } else {
            unreachable!()
        }
    }

    unsafe fn jump_diag<const DX: i32, const DY: i32>(
        &self,
        edges: &mut Vec<(NodeRef<'a>, f64)>,
        mut x: i32,
        mut y: i32,
        mut x_verif: i32,
        mut y_verif: i32,
    ) {
        let mut cost = 0.0;
        loop {
            x += DX;
            y += DY;
            cost += SQRT_2;

            if (x, y) == self.target {
                edges.push((self.node_pool.generate_unchecked((x, y)), cost));
                break;
            }

            let x_t = self.map.map.get_unchecked(x + DX, y);
            let y_t = self.map.map.get_unchecked(x, y + DY);
            let xy_t = self.map.map.get_unchecked(x + DX, y + DY);
            if x_t {
                x_verif = self.jump_x::<DX, DY>(edges, x, y, cost, x_verif);
            }
            if y_t {
                y_verif = self.jump_y::<DX, DY>(edges, x, y, cost, y_verif);
            }
            if !(x_t && y_t && xy_t) {
                break;
            }
        }
    }
}

#[inline(always)]
unsafe fn jump_left_verif<const DY: i32>(
    map: &BitGrid,
    mut x: i32,
    y: i32,
    verif: i32,
) -> (i32, bool) {
    while DY != 0 && x >= verif + 56 {
        let row_adj = map.get_row_west(x, y + DY);
        let row = map.get_row_west(x, y);

        let adj_turning = !row_adj >> 1 & row_adj;
        let stops = (adj_turning | !row) & !0x7F;

        if stops != 0 {
            let dist = stops.leading_zeros() as i32;
            return (x - dist, row & (1 << (63 - dist)) != 0);
        }

        x -= 56;
    }
    loop {
        let row_above = map.get_row_west(x, y - 1);
        let row = map.get_row_west(x, y);
        let row_below = map.get_row_west(x, y + 1);

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

#[inline(always)]
unsafe fn jump_right_verif<const DY: i32>(
    map: &BitGrid,
    mut x: i32,
    y: i32,
    verif: i32,
) -> (i32, bool) {
    while DY != 0 && x <= verif - 56 {
        let row_adj = map.get_row_east(x, y + DY);
        let row = map.get_row_east(x, y);

        let adj_turning = !row_adj << 1 & row_adj;
        let stops = (adj_turning | !row) & ((1 << 57) - 1);

        if stops != 0 {
            let dist = stops.trailing_zeros() as i32;
            return (x + dist, row & 1 << dist != 0);
        }

        x += 56;
    }
    loop {
        let row_above = map.get_row_east(x, y - 1);
        let row = map.get_row_east(x, y);
        let row_below = map.get_row_east(x, y + 1);

        let above_turning = !row_above << 1 & row_above;
        let below_turning = !row_below << 1 & row_below;
        let stops = (above_turning | below_turning | !row) & ((1 << 57) - 1);

        if stops != 0 {
            let dist = stops.trailing_zeros() as i32;
            return (x + dist, row & 1 << dist != 0);
        }

        x += 56;
    }
}
