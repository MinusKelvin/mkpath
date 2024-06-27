use std::io::{Read, Write};

use mkpath_core::traits::{Cost, Expander, NodePool, OpenList, Successor};
use mkpath_core::{NodeBuilder, PriorityQueueFactory};
use mkpath_ess::{ExplicitStateSpace, Mapper};
use rand::Rng;
use rand_pcg::Pcg64;

pub struct DifferentialHeuristic<SS: ExplicitStateSpace, const N: usize> {
    data: SS::Auxiliary<[f64; N]>,
}

impl<SS: ExplicitStateSpace, const N: usize> DifferentialHeuristic<SS, N> {
    pub fn calculate(domain: &SS, mapper: &Mapper<SS>) -> Self
    where
        for<'a> <SS::Expander<'a> as Expander<'a>>::Edge: Successor<'a> + Cost,
    {
        let mut this = Self {
            data: domain.new_auxiliary(|_| [f64::INFINITY; N]),
        };

        let mut rng = Pcg64::new(0xcafef00dd15ea5e5, 0xa02bdbf7bb3c0a7ac28fa16a64abf96);

        let nodes_required = (0..mapper.components())
            .map(|comp| mapper.component_id_range(comp).len())
            .max()
            .unwrap_or(0);

        let mut builder = NodeBuilder::new();
        let state = domain.add_state_field(&mut builder);
        let g = builder.add_field(f64::INFINITY);
        let mut pqueue = PriorityQueueFactory::new(&mut builder);
        let mut pool = domain.new_node_pool(builder.build_with_capacity(nodes_required), state);

        for component in 0..mapper.components() {
            let id_range = mapper.component_id_range(component);
            for i in 0..N {
                let pivot = mapper.to_state(rng.gen_range(id_range.clone()));

                pool.reset();
                let mut queue = pqueue.new_queue(g);
                let mut expander = domain.new_expander(&pool);
                let mut edges = vec![];
                let start = pool.generate(pivot);
                start.set(g, 0.0);
                queue.relaxed(start);

                while let Some(node) = queue.next() {
                    let node_g = node.get(g);
                    this.data[node.get(state)][i] = node_g;

                    edges.clear();
                    expander.expand(node, &mut edges);

                    for edge in &edges {
                        let successor = edge.successor();
                        let new_g = node_g + edge.cost();
                        if new_g < successor.get(g) {
                            successor.set(g, new_g);
                            successor.set_parent(Some(node));
                            queue.relaxed(successor);
                        }
                    }
                }
            }
        }

        this
    }

    pub fn save(&self, mapper: &Mapper<SS>, to: &mut impl Write) -> std::io::Result<()> {
        for id in 0..mapper.states() {
            for d in self.data[mapper.to_state(id)] {
                to.write_all(&d.to_le_bytes())?;
            }
        }
        Ok(())
    }

    pub fn load(domain: &SS, mapper: &Mapper<SS>, from: &mut impl Read) -> std::io::Result<Self> {
        let mut data = domain.new_auxiliary(|_| [f64::INFINITY; N]);
        for id in 0..mapper.states() {
            let data = &mut data[mapper.to_state(id)];
            for i in 0..N {
                let mut buf = [0; 8];
                from.read_exact(&mut buf)?;
                data[i] = f64::from_le_bytes(buf);
            }
        }
        Ok(DifferentialHeuristic { data })
    }

    pub fn h(&self, state: SS::State, goal: SS::State) -> f64 {
        self.partial_h(state, goal, N)
    }

    fn partial_h(&self, state: SS::State, goal: SS::State, n: usize) -> f64 {
        let mut best = 0.0;
        let state = &self.data[state];
        let goal = &self.data[goal];
        for (state, goal) in state.iter().zip(goal.iter()).take(n) {
            let h = (state - goal).abs();
            best = h.max(best);
        }
        best
    }
}
