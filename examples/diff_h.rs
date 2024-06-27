use std::path::PathBuf;

use clap::Parser;
use mkpath::grid::{EightConnectedExpander, GridPool};
use mkpath::traits::NodePool;
use mkpath::{AStarSearcher, NodeAllocator, NodeBuilder, NodeMemberPointer, PriorityQueueFactory};
use mkpath_ess::{ExplicitStateSpace, Mapper};
use mkpath_grid::{BitGrid, Grid};
use mkpath_tdh::DifferentialHeuristic;

mod movingai;

#[derive(Parser)]
struct Options {
    path: PathBuf,
}

struct EightConnectedGrid(BitGrid);

impl ExplicitStateSpace for EightConnectedGrid {
    type State = (i32, i32);

    type Auxiliary<T> = Grid<T>;

    type NodePool = GridPool;

    type Expander<'a> = EightConnectedExpander<'a, GridPool>
    where
        Self: 'a;

    fn new_auxiliary<T>(&self, mut init: impl FnMut(Self::State) -> T) -> Self::Auxiliary<T> {
        Grid::new(self.0.width(), self.0.height(), |x, y| init((x, y)))
    }

    fn add_state_field(&self, builder: &mut NodeBuilder) -> NodeMemberPointer<Self::State> {
        builder.add_field((-1, -1))
    }

    fn new_node_pool(
        &self,
        alloc: NodeAllocator,
        state: NodeMemberPointer<Self::State>,
    ) -> Self::NodePool {
        GridPool::new(alloc, state, self.0.width(), self.0.height())
    }

    fn new_expander<'a>(&'a self, node_pool: &'a Self::NodePool, state: NodeMemberPointer<Self::State>) -> Self::Expander<'a> {
        EightConnectedExpander::new(&self.0, node_pool, state)
    }

    fn list_valid_states(&self) -> Vec<Self::State> {
        let mut res = vec![];
        for y in 0..self.0.height() {
            for x in 0..self.0.width() {
                if self.0.get(x, y) {
                    res.push((x, y));
                }
            }
        }
        res
    }
}

fn main() {
    let opt = Options::parse();

    let t1 = std::time::Instant::now();

    let scen = movingai::read_scenario(&opt.path).unwrap();
    let map = EightConnectedGrid(movingai::read_bitgrid(&scen.map).unwrap());

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
