use std::path::PathBuf;

use clap::Parser;
use mkpath::grid::{EightConnectedExpander, GridPool, EightConnectedDomain};
use mkpath::traits::NodePool;
use mkpath::{AStarSearcher, NodeBuilder, PriorityQueueFactory};
use mkpath_ess::Mapper;
use mkpath_tdh::DifferentialHeuristic;

mod movingai;

#[derive(Parser)]
struct Options {
    path: PathBuf,
}

fn main() {
    let opt = Options::parse();

    let t1 = std::time::Instant::now();

    let scen = movingai::read_scenario(&opt.path).unwrap();
    let map = EightConnectedDomain(movingai::read_bitgrid(&scen.map).unwrap());

    let mapper = Mapper::dfs_preorder(&map);

    let diff_h = DifferentialHeuristic::<_, 8>::calculate(&map, &mapper);

    let mut builder = NodeBuilder::new();
    let state = builder.add_field((-1, -1));
    let mut astar = AStarSearcher::new(&mut builder);
    let mut open_list_factory = PriorityQueueFactory::new(&mut builder);
    let mut pool = GridPool::new(builder.build(), state, map.0.width(), map.0.height());

    let t2 = std::time::Instant::now();

    for problem in &scen.instances {
        pool.reset();

        let open_list = open_list_factory.new_queue(astar.ordering());
        let expander = EightConnectedExpander::new(&map.0, &pool, state);

        let result = astar.search(
            expander,
            open_list,
            |node| diff_h.h(node.get(state), problem.target),
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
