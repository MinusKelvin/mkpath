use std::path::PathBuf;

use mkpath::grid::{octile_distance, GridPool};
use mkpath::jps::CanonicalGridExpander;
use mkpath::{AStarSearcher, NodeBuilder, PriorityQueueFactory};
use structopt::StructOpt;

mod movingai;

#[derive(StructOpt)]
struct Options {
    scen: PathBuf,
}

fn main() {
    let opt = Options::from_args();
    let scen = movingai::read_scenario(&opt.scen).unwrap();
    let map = movingai::read_bitgrid(&scen.map).unwrap();

    let mut builder = NodeBuilder::new();
    let state = builder.add_field((-1, -1));
    let mut astar = AStarSearcher::new(&mut builder);
    let mut open_list_factory = PriorityQueueFactory::new(&mut builder);
    let mut pool = GridPool::new(builder.build(), state, map.width(), map.height());

    for problem in &scen.instances {
        pool.reset();

        let open_list = open_list_factory.new_queue(astar.ordering());
        let expander = CanonicalGridExpander::new(&map, &pool);

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
}
