use std::path::PathBuf;

use clap::Parser;
use mkpath::grid::octile_distance;
use mkpath::jps::{JpsPlusExpander, JumpDatabase};
use mkpath::traits::NodePool;
use mkpath::{AStarSearcher, HashPool, NodeBuilder, PriorityQueueFactory};

mod movingai;

#[derive(Parser)]
struct Options {
    scen: PathBuf,
}

fn main() {
    let opt = Options::parse();

    let t1 = std::time::Instant::now();

    let scen = movingai::read_scenario(&opt.scen).unwrap();
    let map = movingai::read_bitgrid(&scen.map).unwrap();

    let mut builder = NodeBuilder::new();
    let state = builder.add_field((-1, -1));
    let mut astar = AStarSearcher::new(&mut builder);
    let mut open_list_factory = PriorityQueueFactory::new(&mut builder);
    let mut pool = HashPool::new(builder.build(), state);

    let t2 = std::time::Instant::now();

    let jump_db = JumpDatabase::new(&map);

    for problem in &scen.instances {
        pool.reset();

        let open_list = open_list_factory.new_queue(astar.ordering());
        let expander = JpsPlusExpander::new(&map, &jump_db, &pool, state, problem.target);

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
    eprintln!("Load: {:<10.2?} Search: {:.2?}", t2 - t1, t3 - t2);
}
