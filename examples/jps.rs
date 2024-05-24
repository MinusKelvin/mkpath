use std::path::PathBuf;

use mkpath::grid::octile_distance;
use mkpath::jps::JpsExpander;
use mkpath::traits::NodePool;
use mkpath::{AStarSearcher, HashPool, NodeBuilder, PriorityQueueFactory};
use mkpath_jps::transpose;
use structopt::StructOpt;

mod movingai;

#[derive(StructOpt)]
struct Options {
    scen: PathBuf,
}

fn main() {
    let opt = Options::from_args();

    let t1 = std::time::Instant::now();

    let scen = movingai::read_scenario(&opt.scen).unwrap();
    let map = movingai::read_bitgrid(&scen.map).unwrap();

    let mut builder = NodeBuilder::new();
    let state = builder.add_field((-1, -1));
    let mut astar = AStarSearcher::new(&mut builder);
    let mut open_list_factory = PriorityQueueFactory::new(&mut builder);
    let mut pool = HashPool::new(builder.build(), state);

    let tmap = transpose(&map);

    let t2 = std::time::Instant::now();

    for problem in &scen.instances {
        pool.reset();

        let open_list = open_list_factory.new_queue(astar.ordering());
        let expander = JpsExpander::new(&map, &tmap, &pool, state, problem.target);

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
