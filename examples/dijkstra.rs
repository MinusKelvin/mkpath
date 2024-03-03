use std::path::PathBuf;

use mkpath::grid::{EightConnectedExpander, GridPool};
use mkpath::traits::Expander;
use mkpath::{NodeBuilder, PriorityQueueFactory};
use mkpath_grid::GridEdge;
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
    let g = builder.add_field(f64::INFINITY);
    let mut open_list_factory = PriorityQueueFactory::new(&mut builder);

    let mut pool = GridPool::new(builder.build(), state, map.width(), map.height());

    for problem in &scen.instances {
        pool.reset();

        let mut open_list = open_list_factory.new_queue(g);
        let mut expander = EightConnectedExpander::new(&map, &pool);
        let mut edges = vec![];

        // start node
        let start = pool.generate(problem.start);
        start.set(g, 0.0);
        open_list.push(start);

        // target node
        let target = pool.generate(problem.target);

        while let Some(node) = open_list.pop() {
            if node.ptr_eq(target) {
                break;
            }

            edges.clear();
            expander.expand(node, &mut edges);

            for &GridEdge {
                successor, cost, ..
            } in &edges
            {
                let new_g = node.get(g) + cost;
                if new_g < successor.get(g) {
                    successor.set_parent(Some(node));
                    successor.set(g, new_g);
                    open_list.push(successor);
                }
            }
        }

        if target.get(g) < f64::INFINITY {
            let mut node = target;
            let mut path = vec![target.get(state)];
            while let Some(parent) = node.get_parent() {
                node = parent;
                path.push(node.get(state));
            }
            path.reverse();
            println!("{:.2} {path:?}", target.get(g));
        } else {
            println!("failed to find path");
        }
    }
}
