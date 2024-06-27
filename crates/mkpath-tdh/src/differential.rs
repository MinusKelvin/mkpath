use std::io::{Read, Write};

use mkpath_core::traits::{Cost, Expander, Successor};
use mkpath_ess::{ExplicitStateSpace, Mapper};
use rand::Rng;
use rand_pcg::Pcg64;

use crate::Searcher;

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

        let mut searcher = Searcher::new(domain, nodes_required);

        for component in 0..mapper.components() {
            let id_range = mapper.component_id_range(component);
            for i in 0..N {
                let mut pivot = mapper.to_state(rng.gen_range(id_range.clone()));
                let mut dist = 0.0;
                if i == 0 {
                    searcher.search(domain, pivot, |state, g| {
                        if g > dist {
                            dist = g;
                            pivot = state;
                        }
                    });
                } else {
                    for id in id_range.clone() {
                        let state = mapper.to_state(id);
                        let d = this.data[state]
                            .iter()
                            .fold(f64::INFINITY, |prev, &new| prev.min(new));
                        if d > dist {
                            dist = d;
                            pivot = state;
                        }
                    }
                }

                searcher.search(domain, pivot, |state, g| {
                    this.data[state][i] = g;
                });
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
