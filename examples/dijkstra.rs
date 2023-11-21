use mkpath::grid::{BitGrid, BitGridExpander, GridPool};
use mkpath::node::NodeBuilder;
use mkpath::pqueue::PriorityQueueFactory;

fn main() {
    let mut builder = NodeBuilder::new();
    let state = builder.add_field((-1, -1));
    let g = builder.add_field(f64::INFINITY);
    let mut open_list_factory = PriorityQueueFactory::new(&mut builder);

    let mut pool = GridPool::new(builder.build(), state, 8, 8);

    let mut map = BitGrid::new(8, 8);
    for y in 0..8 {
        for x in 0..8 {
            map.set(x, y, x != 4 || y >= 6);
        }
    }

    for y in (0..8).rev() {
        for x in 0..8 {
            if map.get(x, y) {
                print!(".")
            } else {
                print!("#")
            }
        }
        println!();
    }

    for i in 0..std::hint::black_box(1) {
        pool.reset();

        let mut open_list = open_list_factory.new_queue(g);
        let mut expander = BitGridExpander::new(&map, &pool);
        let mut edges = vec![];

        // start node
        let start = pool.generate(2, 2);
        start.set(g, 0.0);
        open_list.push(start);

        // target node
        let target = pool.generate(6, 2);

        while let Some(node) = open_list.pop() {
            if node.same_ptr(target) {
                break;
            }

            edges.clear();
            expander.expand(node, &mut edges);

            for &(successor, cost) in &edges {
                let new_g = node.get(g) + cost;
                if new_g < successor.get(g) {
                    successor.set_parent(Some(node));
                    successor.set(g, new_g);
                    open_list.push(successor);
                }
            }
        }

        if target.get(g) < f64::INFINITY {
            println!("found path");
            let mut node = target;
            let mut path = vec![target.get(state)];
            while let Some(parent) = node.get_parent() {
                node = parent;
                path.push(node.get(state));
            }
            path.reverse();
            println!("{path:?}");
        } else {
            println!("failed to find path");
        }
    }
}
