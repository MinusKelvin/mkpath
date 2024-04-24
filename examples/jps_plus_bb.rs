use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

use mkpath::{AStarSearcher, HashPool, NodeBuilder, PriorityQueueFactory};
use mkpath_grid::octile_distance;
use mkpath_grid_gb::{JpsBbExpander, PartialCellBb};
use mkpath_jps::JumpDatabase;
use structopt::StructOpt;

mod movingai;

#[derive(StructOpt)]
struct Options {
    path: PathBuf,
    #[structopt(long)]
    generate: bool,
}

fn main() {
    let opt = Options::from_args();

    if opt.generate {
        let mut cpd_file = opt.path.clone();
        cpd_file.as_mut_os_string().push(".bb+");

        let map = movingai::read_bitgrid(&opt.path).unwrap();
        let jump_db = JumpDatabase::new(&map);

        let oracle = PartialCellBb::compute(&map, &jump_db, |progress, total, time| {
            let done = progress == total;
            let progress = progress as f64 / total as f64;
            let ttg = if done {
                time.as_secs_f64() as u64
            } else {
                (time.as_secs_f64() / progress - time.as_secs_f64()) as u64
            };
            let mut stdout = std::io::stdout().lock();
            let _ = write!(
                stdout,
                "\r{:4.1}% {} {} hr {:2} min {:2} sec",
                (progress * 1000.0).round() / 10.0,
                if done { "Done" } else { "ETA" },
                ttg / 60 / 60,
                ttg / 60 % 60,
                ttg % 60,
            );
            stdout.flush().unwrap();
        });
        println!();

        oracle
            .save(&mut BufWriter::new(File::create(cpd_file).unwrap()))
            .unwrap();
    } else {
        let t1 = std::time::Instant::now();

        let scen = movingai::read_scenario(&opt.path).unwrap();
        let map = movingai::read_bitgrid(&scen.map).unwrap();
        let jump_db = JumpDatabase::new(&map);

        let mut cpd_file = scen.map.clone();
        cpd_file.as_mut_os_string().push(".bb+");
        let oracle =
            PartialCellBb::load(&map, &mut BufReader::new(File::open(cpd_file).unwrap())).unwrap();

        let mut builder = NodeBuilder::new();
        let state = builder.add_field((-1, -1));
        let mut astar = AStarSearcher::new(&mut builder);
        let mut open_list_factory = PriorityQueueFactory::new(&mut builder);
        let mut pool = HashPool::new(builder.build(), state);

        let t2 = std::time::Instant::now();

        for problem in &scen.instances {
            pool.reset();

            let open_list = open_list_factory.new_queue(astar.ordering());
            let expander = JpsBbExpander::new(&map, &jump_db, &oracle, &pool, problem.target);

            let result = astar.search(
                expander,
                open_list,
                |node| octile_distance(node.get(state), problem.target),
                |node| node.get(state) == problem.target,
                pool.generate(problem.start),
            );

            if let Some(path) = result {
                let cost = path.last().unwrap().get(astar.g());
                let path: Vec<_> = path.into_iter().map(|node| node.get(state)).collect();
                println!("{cost:.2} {path:?}");
            } else {
                println!("failed to find path");
            }
        }

        let t3 = std::time::Instant::now();
        eprintln!("Load: {:<10.2?} Search: {:<10.2?}", t2 - t1, t3 - t2,);
    }
}
