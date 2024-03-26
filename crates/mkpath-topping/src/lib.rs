use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use ahash::HashMap;
use enumset::EnumSet;
use mkpath_core::traits::{Expander, OpenList};
use mkpath_core::NodeBuilder;
use mkpath_cpd::{BucketQueueFactory, CpdRow, StateIdMapper};
use mkpath_grid::{BitGrid, Direction, EightConnectedExpander, Grid, GridPool};
use mkpath_jps::{canonical_successors, JumpDatabase};
use rayon::prelude::*;

mod tops_expander;

pub use self::tops_expander::*;

pub struct ToppingPlusOracle {
    mapper: GridMapper,
    jump_db: JumpDatabase,
    partial_cpd: HashMap<(i32, i32), CpdRow>,
}

impl ToppingPlusOracle {
    pub fn compute(
        map: BitGrid,
        progress_callback: impl Fn(usize, usize, Duration) + Sync,
    ) -> Self {
        use Direction::*;

        let jump_db = JumpDatabase::new(map);
        let map = jump_db.map();
        let mapper = GridMapper::dfs_preorder(map);

        let diagonals = NorthWest | SouthWest | NorthEast | SouthEast;

        let mut jump_points = HashMap::default();
        for y in 0..map.height() {
            for x in 0..map.width() {
                if !map.get(x, y) {
                    continue;
                }

                let nb = map.get_neighborhood(x, y);
                let mut jp_successors = EnumSet::empty();
                let mut jps = EnumSet::empty();

                for dir in [North, South, East, West] {
                    if !nb.contains(dir.backwards()) {
                        continue;
                    }
                    let dirs = canonical_successors(nb, Some(dir));
                    if dirs & dir != dirs {
                        jps |= dir;
                        jp_successors |= dirs;
                    }
                }

                if !jps.is_empty() {
                    *jump_points.entry((x, y)).or_default() |= jps;

                    jp_successors &= diagonals;
                    if jp_successors.contains(NorthWest) {
                        collect_diagonal_jps(&mut jump_points, &jump_db, x, y, NorthWest);
                    }
                    if jp_successors.contains(SouthWest) {
                        collect_diagonal_jps(&mut jump_points, &jump_db, x, y, SouthWest);
                    }
                    if jp_successors.contains(SouthEast) {
                        collect_diagonal_jps(&mut jump_points, &jump_db, x, y, SouthEast);
                    }
                    if jp_successors.contains(NorthEast) {
                        collect_diagonal_jps(&mut jump_points, &jump_db, x, y, NorthEast);
                    }
                }
            }
        }

        let progress = AtomicUsize::new(0);
        let start = std::time::Instant::now();
        let num_jps = jump_points.len();

        let partial_cpd: HashMap<_, _> = jump_points
            .par_iter()
            .map_init(
                || {
                    let mut builder = NodeBuilder::new();
                    let state = builder.add_field((-1, -1));
                    let successors = builder.add_field(EnumSet::all());
                    let first_move = builder.add_field(EnumSet::all());
                    let g = builder.add_field(f64::INFINITY);
                    let pqueue = BucketQueueFactory::new(&mut builder);
                    let pool = GridPool::new(
                        builder.build_with_capacity(mapper.array.len()),
                        state,
                        map.width(),
                        map.height(),
                    );
                    (state, successors, g, first_move, pqueue, pool)
                },
                |&mut (state, successors, g, first_move, ref pqueue, ref mut pool),
                 (&source, &jps)| {
                    pool.reset();

                    let mut first_moves = vec![EnumSet::all(); mapper.num_ids()];
                    let mut edges = vec![];
                    let mut expander = EightConnectedExpander::new(&map, pool);
                    let mut open = pqueue.new_queue(g, 0.999);
                    let tiebreak_table =
                        compute_tiebreak_table(map.get_neighborhood(source.0, source.1), jps);

                    let start_node = pool.generate(source);
                    start_node.set(g, 0.0);

                    expander.expand(start_node, &mut edges);
                    for edge in &edges {
                        let node = edge.successor;
                        node.set(g, edge.cost);
                        node.set(first_move, EnumSet::only(edge.direction));
                        node.set_parent(Some(start_node));
                        let (x, y) = node.get(state);
                        node.set(
                            successors,
                            canonical_successors(map.get_neighborhood(x, y), Some(edge.direction)),
                        );
                        open.relaxed(node);
                    }

                    while let Some(node) = open.next() {
                        first_moves[mapper.state_to_id(node.get(state))] =
                            tiebreak_table[node.get(first_move).as_usize()];
                        edges.clear();
                        expander.expand(node, &mut edges);
                        for edge in &edges {
                            if !node.get(successors).contains(edge.direction) {
                                continue;
                            }
                            let successor = edge.successor;
                            let (x, y) = successor.get(state);
                            let new_g = edge.cost + node.get(g);
                            // TODO: think about floating point round-off error
                            if new_g < successor.get(g) {
                                // Shorter path to node; overwrite first move and successors.
                                successor.set(g, new_g);
                                successor.set(first_move, node.get(first_move));
                                successor.set(
                                    successors,
                                    canonical_successors(
                                        map.get_neighborhood(x, y),
                                        Some(edge.direction),
                                    ),
                                );
                                successor.set_parent(Some(node));
                                open.relaxed(successor);
                            } else if new_g == successor.get(g) {
                                // In case of tie, multiple first moves may allow optimal paths.
                                // Additionally, there are more canonical successors to consider
                                // when the node is expanded.
                                successor.set(
                                    first_move,
                                    successor.get(first_move) | node.get(first_move),
                                );
                                successor.set(
                                    successors,
                                    successor.get(successors)
                                        | canonical_successors(
                                            map.get_neighborhood(x, y),
                                            Some(edge.direction),
                                        ),
                                );
                            }
                        }
                    }

                    let result = CpdRow::compress(first_moves.into_iter().map(|set| set.as_u64()));

                    let progress = progress.fetch_add(1, Ordering::Relaxed) + 1;
                    progress_callback(progress, num_jps, start.elapsed());
                    (source, result)
                },
            )
            .collect();

        ToppingPlusOracle {
            mapper,
            jump_db,
            partial_cpd,
        }
    }

    pub fn load(map: BitGrid, from: &mut impl Read) -> std::io::Result<Self> {
        let jump_db = JumpDatabase::new(map);
        let mapper = GridMapper::load(from)?;

        let mut bytes = [0; 4];
        from.read_exact(&mut bytes)?;
        let num_jps = u32::from_le_bytes(bytes) as usize;

        let mut partial_cpd = HashMap::default();
        for _ in 0..num_jps {
            from.read_exact(&mut bytes)?;
            let x = i32::from_le_bytes(bytes);
            from.read_exact(&mut bytes)?;
            let y = i32::from_le_bytes(bytes);

            assert!(x >= 0);
            assert!(y >= 0);
            assert!(x < jump_db.map().width());
            assert!(y < jump_db.map().height());

            partial_cpd.insert((x, y), CpdRow::load(from)?);
        }

        Ok(ToppingPlusOracle {
            mapper,
            jump_db,
            partial_cpd,
        })
    }

    pub fn save(&self, to: &mut impl Write) -> std::io::Result<()> {
        self.mapper.save(to)?;
        to.write_all(&u32::to_le_bytes(self.partial_cpd.len() as u32))?;
        for ((x, y), row) in &self.partial_cpd {
            to.write_all(&x.to_le_bytes())?;
            to.write_all(&y.to_le_bytes())?;
            row.save(to)?;
        }
        Ok(())
    }

    pub fn query(&self, pos: (i32, i32), target: (i32, i32)) -> Option<Direction> {
        self.partial_cpd
            .get(&pos)
            .and_then(|row| row.lookup(self.mapper.state_to_id(target)).try_into().ok())
    }
}

fn collect_diagonal_jps(
    jump_points: &mut HashMap<(i32, i32), EnumSet<Direction>>,
    jump_db: &JumpDatabase,
    mut x: i32,
    mut y: i32,
    dir: Direction,
) {
    let (dx, dy) = match dir {
        Direction::NorthWest => (-1, -1),
        Direction::SouthWest => (-1, 1),
        Direction::SouthEast => (1, 1),
        Direction::NorthEast => (1, -1),
        _ => unreachable!(),
    };

    while let (dist, true) = jump_db.get(x, y, dir) {
        x += dx * dist;
        y += dy * dist;
        *jump_points.entry((x, y)).or_default() |= dir;
    }
}

pub struct GridMapper {
    grid: Grid<usize>,
    array: Box<[(i32, i32)]>,
}

impl GridMapper {
    pub fn dfs_preorder(map: &BitGrid) -> Self {
        let mut grid = Grid::new(map.width(), map.height(), |_, _| usize::MAX);
        let mut array = vec![];

        let mut builder = NodeBuilder::new();
        let state = builder.add_field((-1, -1));
        let mut pool = GridPool::new(builder.build(), state, map.width(), map.height());

        for y in 0..map.height() {
            for x in 0..map.width() {
                if !map.get(x, y) || grid[(x, y)] != usize::MAX {
                    continue;
                }

                pool.reset();
                mkpath_cpd::dfs_traversal(
                    pool.generate((x, y)),
                    EightConnectedExpander::new(&map, &pool),
                    |node| {
                        if grid[node.get(state)] == usize::MAX {
                            grid[node.get(state)] = array.len();
                            array.push(node.get(state));
                            true
                        } else {
                            false
                        }
                    },
                );
            }
        }

        GridMapper {
            grid,
            array: array.into_boxed_slice(),
        }
    }

    pub fn load(from: &mut impl Read) -> std::io::Result<Self> {
        let mut bytes = [0; 4];
        from.read_exact(&mut bytes)?;
        let len = u32::from_le_bytes(bytes) as usize;

        from.read_exact(&mut bytes)?;
        let width = i32::from_le_bytes(bytes);
        from.read_exact(&mut bytes)?;
        let height = i32::from_le_bytes(bytes);

        let mut grid = Grid::new(width, height, |_, _| usize::MAX);
        let mut array = vec![(0, 0); len].into_boxed_slice();
        for id in 0..len {
            from.read_exact(&mut bytes)?;
            let x = i32::from_le_bytes(bytes);
            from.read_exact(&mut bytes)?;
            let y = i32::from_le_bytes(bytes);
            grid[(x, y)] = id;
            array[id] = (x, y);
        }

        Ok(GridMapper { grid, array })
    }

    pub fn save(&self, to: &mut impl Write) -> std::io::Result<()> {
        to.write_all(&(self.array.len() as u32).to_le_bytes())?;
        to.write_all(&self.grid.width().to_le_bytes())?;
        to.write_all(&self.grid.height().to_le_bytes())?;
        for (x, y) in self.array.iter() {
            to.write_all(&x.to_le_bytes())?;
            to.write_all(&y.to_le_bytes())?;
        }
        Ok(())
    }
}

impl StateIdMapper for GridMapper {
    type State = (i32, i32);

    fn num_ids(&self) -> usize {
        self.array.len()
    }

    fn state_to_id(&self, state: Self::State) -> usize {
        self.grid[state]
    }

    fn id_to_state(&self, id: usize) -> Self::State {
        self.array[id]
    }
}

fn is_irrelevant_jp(jp: Direction, fm: EnumSet<Direction>, nb: EnumSet<Direction>) -> bool {
    use Direction::*;

    let canonical = canonical_successors(nb, Some(jp));
    // Simple non-optimal/non-canonical case
    if canonical.is_disjoint(fm) {
        return true;
    }

    // Cases 1, 2, 3, and 4 (backwards, switchback, and diagonal-to-diagonal turn)
    if !fm.is_disjoint(match jp {
        North => SouthWest | South | SouthEast,
        West => NorthEast | East | SouthEast,
        South => NorthWest | North | NorthEast,
        East => NorthWest | West | SouthWest,
        NorthWest => SouthWest | South | SouthEast | East | NorthEast,
        SouthWest => SouthEast | East | NorthEast | North | NorthWest,
        SouthEast => NorthEast | North | NorthWest | West | SouthWest,
        NorthEast => NorthWest | West | SouthWest | South | SouthEast,
    }) {
        return true;
    }

    // Case 5 (orthogonal-to-orthogonal turn)
    match jp {
        North if fm.contains(West) && nb.contains(SouthWest) => return true,
        North if fm.contains(East) && nb.contains(SouthEast) => return true,
        West if fm.contains(South) && nb.contains(SouthEast) => return true,
        West if fm.contains(North) && nb.contains(NorthEast) => return true,
        South if fm.contains(West) && nb.contains(NorthWest) => return true,
        South if fm.contains(East) && nb.contains(NorthEast) => return true,
        East if fm.contains(South) && nb.contains(SouthWest) => return true,
        East if fm.contains(North) && nb.contains(NorthWest) => return true,
        _ => {}
    }

    false
}

fn compute_tiebreak_table(
    nb: EnumSet<Direction>,
    jps: EnumSet<Direction>,
) -> [EnumSet<Direction>; 256] {
    let valid_moves = canonical_successors(nb, None);
    let mut result = [EnumSet::empty(); 256];
    // empty first move set is invalid, so skip it
    for fm in 1..256 {
        let fm_dirs = EnumSet::from_u8(fm as u8);
        result[fm] = fm_dirs;

        if !fm_dirs.is_subset(valid_moves) {
            // first move set is invalid because it contains illegal moves; skip
            continue;
        }

        for jp in jps {
            if is_irrelevant_jp(jp, fm_dirs, nb) {
                continue;
            }
            result[fm] &= canonical_successors(nb, Some(jp));
        }

        assert!(!result[fm].is_empty());
    }
    result
}

#[test]
fn tiebreaking_is_valid() {
    use Direction::*;

    for nb in 0..256 {
        let nb = EnumSet::from_u8(nb as u8);

        let mut jp_successors = EnumSet::empty();
        let mut jp = EnumSet::empty();

        // find orthogonal jump points
        for dir in [North, South, East, West] {
            if !nb.contains(dir.backwards()) {
                continue;
            }
            let successors = canonical_successors(nb, Some(dir));
            if successors & dir != successors {
                jp_successors |= successors;
                jp |= dir;
            }
        }

        // find potential diagonal jump points
        for dir in [NorthWest, NorthEast, SouthWest, SouthEast] {
            let dir_x = match dir {
                NorthWest | SouthWest => West,
                _ => East,
            };
            let dir_y = match dir {
                NorthWest | NorthEast => North,
                _ => South,
            };

            if nb.is_superset(dir_x.backwards() | dir_y.backwards() | dir.backwards())
                && !nb.is_disjoint(dir_x | dir_y)
            {
                // possible diagonal jump point
                // note that since more jump points can only make the valid first move set smaller,
                // we don't need to check the same neighborhood with fewer diagonal jump points
                jp |= dir;
                jp_successors |= canonical_successors(nb, Some(dir));
            }
        }

        if jp.is_empty() {
            continue;
        }

        // computation of tiebreak table checks the non-empty invariant
        compute_tiebreak_table(nb, jp);
    }
}
