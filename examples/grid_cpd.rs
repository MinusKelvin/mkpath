use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use clap::Parser;
use mkpath::cpd::{CpdRow, FirstMoveSearcher, StateIdMapper};
use mkpath::grid::{EightConnectedExpander, Grid, GridPool};
use mkpath::traits::NodePool;
use mkpath::NodeBuilder;
use mkpath_grid::bucket_queue::BucketQueueFactory;
use mkpath_grid::{Direction, SAFE_SQRT_2};
use rayon::prelude::*;

mod movingai;

#[derive(Parser)]
struct Options {
    path: PathBuf,
    #[arg(long)]
    generate: bool,
}

fn main() {
    let opt = Options::parse();

    if opt.generate {
        let mut cpd_file = opt.path.clone();
        cpd_file.as_mut_os_string().push(".mkp-cpd");
        build_cpd(&opt.path, &cpd_file).unwrap();
    } else {
        let t1 = std::time::Instant::now();

        let scen = movingai::read_scenario(&opt.path).unwrap();
        let map = movingai::read_bitgrid(&scen.map).unwrap();
        let mut cpd_file = scen.map.clone();
        cpd_file.as_mut_os_string().push(".mkp-cpd");
        let (mapper, rows) = load_cpd(&cpd_file, map.width(), map.height()).unwrap();

        let t2 = std::time::Instant::now();

        for problem in scen.instances {
            let mut cost = 0.0;
            let mut path = vec![problem.start];
            let target_id = mapper.state_to_id(problem.target);

            while let Some(&state) = path.last() {
                if state == problem.target {
                    break;
                }
                let dir = rows[mapper.state_to_id(state)].lookup(target_id);
                match Direction::try_from(dir).unwrap() {
                    Direction::North => path.push((state.0, state.1 - 1)),
                    Direction::West => path.push((state.0 - 1, state.1)),
                    Direction::South => path.push((state.0, state.1 + 1)),
                    Direction::East => path.push((state.0 + 1, state.1)),
                    Direction::NorthWest => path.push((state.0 - 1, state.1 - 1)),
                    Direction::SouthWest => path.push((state.0 - 1, state.1 + 1)),
                    Direction::SouthEast => path.push((state.0 + 1, state.1 + 1)),
                    Direction::NorthEast => path.push((state.0 + 1, state.1 - 1)),
                }
                if dir > 3 {
                    cost += SAFE_SQRT_2;
                } else {
                    cost += 1.0;
                }
            }

            println!("{cost:.2} {path:?}");
        }

        let t3 = std::time::Instant::now();
        eprintln!("Load: {:<10.2?} Search: {:.2?}", t2 - t1, t3 - t2);
    }
}

struct GridMapper {
    grid: Grid<usize>,
    array: Vec<(i32, i32)>,
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

fn build_cpd(map: &Path, output: &Path) -> std::io::Result<()> {
    let map = movingai::read_bitgrid(map)?;

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
            mkpath::cpd::dfs_traversal(
                pool.generate((x, y)),
                EightConnectedExpander::new(&map, &pool, state),
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

    drop(pool);

    let mapper = GridMapper { grid, array };
    let progress = AtomicUsize::new(0);
    let mut rows = vec![];

    let t = std::time::Instant::now();
    mapper
        .array
        .par_iter()
        .map_init(
            || {
                let mut builder = NodeBuilder::new();
                let state = builder.add_field((-1, -1));
                let searcher = FirstMoveSearcher::new(&mut builder);
                let pqueue = BucketQueueFactory::new(&mut builder);
                let pool = GridPool::new(
                    builder.build_with_capacity(mapper.array.len()),
                    state,
                    map.width(),
                    map.height(),
                );
                (state, searcher, pqueue, pool)
            },
            |(state, searcher, pqueue, pool), &source| {
                pool.reset();
                let result = CpdRow::compute(
                    &mapper,
                    searcher,
                    EightConnectedExpander::new(&map, pool, *state),
                    pqueue.new_queue(searcher.g(), 0.9),
                    pool.generate(source),
                    *state,
                );
                let progress = progress.fetch_add(1, Ordering::SeqCst) + 1;
                let progress = progress as f64 / mapper.num_ids() as f64;
                let d = t.elapsed();
                let ttg = (d.as_secs_f64() / progress - d.as_secs_f64()) as u64;
                print!(
                    "\r{:4.1}% ETA {} hr {:2} min {:2} sec",
                    (progress * 1000.0).round() / 10.0,
                    ttg / 60 / 60,
                    ttg / 60 % 60,
                    ttg % 60,
                );
                std::io::stdout().flush().unwrap();
                result
            },
        )
        .collect_into_vec(&mut rows);

    let mut output = BufWriter::new(File::create(output)?);

    output.write_all(&(mapper.array.len() as u32).to_le_bytes())?;
    for (x, y) in mapper.array {
        output.write_all(&x.to_le_bytes())?;
        output.write_all(&y.to_le_bytes())?;
    }
    output.write_all(&0xA53BE83Fu32.to_le_bytes())?;
    for row in rows {
        row.save(&mut output)?;
    }

    Ok(())
}

fn load_cpd(
    cpd_file: &Path,
    width: i32,
    height: i32,
) -> std::io::Result<(GridMapper, Vec<Box<CpdRow>>)> {
    let mut cpd_file = BufReader::new(File::open(cpd_file)?);

    let mut bytes = [0; 4];
    cpd_file.read_exact(&mut bytes)?;
    let len = u32::from_le_bytes(bytes) as usize;

    let mut grid = Grid::new(width, height, |_, _| usize::MAX);
    let mut array = vec![(0, 0); len];
    for id in 0..len {
        cpd_file.read_exact(&mut bytes)?;
        let x = i32::from_le_bytes(bytes);
        cpd_file.read_exact(&mut bytes)?;
        let y = i32::from_le_bytes(bytes);
        grid[(x, y)] = id;
        array[id] = (x, y);
    }

    cpd_file.read_exact(&mut bytes)?;
    assert_eq!(u32::from_le_bytes(bytes), 0xA53BE83F);

    let rows = (0..len)
        .map(|_| CpdRow::load(&mut cpd_file))
        .collect::<std::io::Result<_>>()?;

    Ok((GridMapper { grid, array }, rows))
}
