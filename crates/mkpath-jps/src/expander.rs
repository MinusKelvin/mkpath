use mkpath_core::NodeRef;
use mkpath_grid::GridStateMapper;

use crate::{Direction, JumpPointLocator};

/// Jump point search expander.
///
/// Harabor, D., & Grastien, A. (2014, May). Improving jump point search. In Proceedings of the
/// International Conference on Automated Planning and Scheduling (Vol. 24, pp. 128-135).
pub(crate) struct GenericJpsExpander<'a, J, P> {
    jpl: J,
    node_pool: &'a P,
}

impl<'a, J: JumpPointLocator, P: GridStateMapper> GenericJpsExpander<'a, J, P> {
    pub fn new(jpl: J, node_pool: &'a P) -> Self {
        // Establish invariant that coordinates in-bounds of the map are also in-bounds of the
        // node pool.
        assert!(
            node_pool.width() >= jpl.map().width(),
            "node pool must be wide enough for the map"
        );
        assert!(
            node_pool.height() >= jpl.map().height(),
            "node pool must be tall enough for the map"
        );

        GenericJpsExpander { jpl, node_pool }
    }

    pub fn expand(&mut self, node: NodeRef, edges: &mut Vec<(NodeRef<'a>, f64)>) {
        let (x, y) = node.get(self.node_pool.state_member());

        let dir = node.get_parent().and_then(|parent| {
            let (px, py) = parent.get(self.node_pool.state_member());
            crate::reached_direction((px, py), (x, y))
        });

        assert!(
            self.jpl.map().get(x, y),
            "attempt to expand node at untraversable location"
        );

        let found = &mut |state, cost| unsafe {
            edges.push((self.node_pool.generate_unchecked(state), cost));
        };

        unsafe {
            // Since x, y is traversable, these are all padded in-bounds, as required.
            let nw = self.jpl.map().get_unchecked(x - 1, y - 1);
            let n = self.jpl.map().get_unchecked(x, y - 1);
            let ne = self.jpl.map().get_unchecked(x + 1, y - 1);
            let w = self.jpl.map().get_unchecked(x - 1, y);
            let e = self.jpl.map().get_unchecked(x + 1, y);
            let sw = self.jpl.map().get_unchecked(x - 1, y + 1);
            let s = self.jpl.map().get_unchecked(x, y + 1);
            let se = self.jpl.map().get_unchecked(x + 1, y + 1);

            // All jumps have the traversability of the relevant tile checked before calling them.
            // Remaining preconditions hold trivially.

            match dir {
                Some(Direction::North) => {
                    let mut north_all_1s = y;
                    if n {
                        north_all_1s = self.jpl.jump_y::<0, -1>(found, x, y, 0.0, 0);
                    }
                    if !sw && w {
                        let west_all_1s = self.jpl.jump_x::<-1, 0>(found, x, y, 0.0, 0);
                        if n && nw {
                            self.jpl
                                .jump_diag::<-1, -1>(found, x, y, west_all_1s, north_all_1s);
                        }
                    }
                    if !se && e {
                        let east_all_1s = self.jpl.jump_x::<1, 0>(found, x, y, 0.0, 0);
                        if n && ne {
                            self.jpl
                                .jump_diag::<1, -1>(found, x, y, east_all_1s, north_all_1s);
                        }
                    }
                }
                Some(Direction::West) => {
                    let mut west_all_1s = x;
                    if w {
                        west_all_1s = self.jpl.jump_x::<-1, 0>(found, x, y, 0.0, 0);
                    }
                    if !ne && n {
                        let north_all_1s = self.jpl.jump_y::<0, -1>(found, x, y, 0.0, 0);
                        if w && nw {
                            self.jpl
                                .jump_diag::<-1, -1>(found, x, y, west_all_1s, north_all_1s);
                        }
                    }
                    if !se && s {
                        let south_all_1s = self.jpl.jump_y::<0, 1>(found, x, y, 0.0, 0);
                        if w && sw {
                            self.jpl
                                .jump_diag::<-1, 1>(found, x, y, west_all_1s, south_all_1s);
                        }
                    }
                }
                Some(Direction::South) => {
                    let mut south_all_1s = y;
                    if s {
                        south_all_1s = self.jpl.jump_y::<0, 1>(found, x, y, 0.0, 0);
                    }
                    if !nw && w {
                        let west_all_1s = self.jpl.jump_x::<-1, 0>(found, x, y, 0.0, 0);
                        if s && sw {
                            self.jpl
                                .jump_diag::<-1, 1>(found, x, y, west_all_1s, south_all_1s);
                        }
                    }
                    if !ne && e {
                        let east_all_1s = self.jpl.jump_x::<1, 0>(found, x, y, 0.0, 0);
                        if s && se {
                            self.jpl
                                .jump_diag::<1, 1>(found, x, y, east_all_1s, south_all_1s);
                        }
                    }
                }
                Some(Direction::East) => {
                    let mut east_all_1s = x;
                    if e {
                        east_all_1s = self.jpl.jump_x::<1, 0>(found, x, y, 0.0, 0);
                    }
                    if !nw && n {
                        let north_all_1s = self.jpl.jump_y::<0, -1>(found, x, y, 0.0, 0);
                        if e && ne {
                            self.jpl
                                .jump_diag::<1, -1>(found, x, y, east_all_1s, north_all_1s);
                        }
                    }
                    if !sw && s {
                        let south_all_1s = self.jpl.jump_y::<0, 1>(found, x, y, 0.0, 0);
                        if e && se {
                            self.jpl
                                .jump_diag::<1, 1>(found, x, y, east_all_1s, south_all_1s);
                        }
                    }
                }
                Some(Direction::NorthWest) => {
                    let mut north_all_1s = y;
                    let mut west_all_1s = x;
                    if n {
                        north_all_1s = self.jpl.jump_y::<0, -1>(found, x, y, 0.0, 0);
                    }
                    if w {
                        west_all_1s = self.jpl.jump_x::<-1, 0>(found, x, y, 0.0, 0);
                    }
                    if n && w && nw {
                        self.jpl
                            .jump_diag::<-1, -1>(found, x, y, west_all_1s, north_all_1s);
                    }
                }
                Some(Direction::SouthWest) => {
                    let mut south_all_1s = y;
                    let mut west_all_1s = x;
                    if s {
                        south_all_1s = self.jpl.jump_y::<0, 1>(found, x, y, 0.0, 0);
                    }
                    if w {
                        west_all_1s = self.jpl.jump_x::<-1, 0>(found, x, y, 0.0, 0);
                    }
                    if s && w && sw {
                        self.jpl
                            .jump_diag::<-1, 1>(found, x, y, west_all_1s, south_all_1s);
                    }
                }
                Some(Direction::SouthEast) => {
                    let mut south_all_1s = y;
                    let mut east_all_1s = x;
                    if s {
                        south_all_1s = self.jpl.jump_y::<0, 1>(found, x, y, 0.0, 0);
                    }
                    if e {
                        east_all_1s = self.jpl.jump_x::<1, 0>(found, x, y, 0.0, 0);
                    }
                    if s && e && se {
                        self.jpl
                            .jump_diag::<1, 1>(found, x, y, east_all_1s, south_all_1s);
                    }
                }
                Some(Direction::NorthEast) => {
                    let mut north_all_1s = y;
                    let mut east_all_1s = x;
                    if n {
                        north_all_1s = self.jpl.jump_y::<0, -1>(found, x, y, 0.0, 0);
                    }
                    if e {
                        east_all_1s = self.jpl.jump_x::<1, 0>(found, x, y, 0.0, 0);
                    }
                    if n && e && ne {
                        self.jpl
                            .jump_diag::<1, -1>(found, x, y, east_all_1s, north_all_1s);
                    }
                }
                None => {
                    let mut north_all_1s = y;
                    let mut south_all_1s = y;
                    let mut east_all_1s = x;
                    let mut west_all_1s = x;
                    if n {
                        north_all_1s = self.jpl.jump_y::<0, -1>(found, x, y, 0.0, 0);
                    }
                    if w {
                        west_all_1s = self.jpl.jump_x::<-1, 0>(found, x, y, 0.0, 0);
                    }
                    if s {
                        south_all_1s = self.jpl.jump_y::<0, 1>(found, x, y, 0.0, 0);
                    }
                    if e {
                        east_all_1s = self.jpl.jump_x::<1, 0>(found, x, y, 0.0, 0);
                    }
                    if n && w && nw {
                        self.jpl
                            .jump_diag::<-1, -1>(found, x, y, west_all_1s, north_all_1s);
                    }
                    if s && w && sw {
                        self.jpl
                            .jump_diag::<-1, 1>(found, x, y, west_all_1s, south_all_1s);
                    }
                    if s && e && se {
                        self.jpl
                            .jump_diag::<1, 1>(found, x, y, east_all_1s, south_all_1s);
                    }
                    if n && e && ne {
                        self.jpl
                            .jump_diag::<1, -1>(found, x, y, east_all_1s, north_all_1s);
                    }
                }
            }
        }
    }
}
