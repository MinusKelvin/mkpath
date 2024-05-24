use std::io::{Read, Write};

use mkpath_core::NodeBuilder;
use mkpath_core::traits::NodePool;
use mkpath_cpd::StateIdMapper;
use mkpath_grid::{BitGrid, EightConnectedExpander, Grid, GridPool};

pub struct GridMapper {
    grid: Grid<usize>,
    array: Box<[(i32, i32)]>,
}

impl GridMapper {
    pub fn dfs_preorder(map: &BitGrid) -> Self {
        let mut grid = Grid::new(map.width(), map.height(), |_, _| usize::MAX);
        let mut array = vec![];

        let mut builder = NodeBuilder::new();
        let state = builder.add_field((-1, -1));
        let mut pool = GridPool::new(builder.build(), state, map.width(), map.height());

        for y in 0..map.height() {
            for x in 0..map.width() {
                if !map.get(x, y) || grid[(x, y)] != usize::MAX {
                    continue;
                }

                pool.reset();
                mkpath_cpd::dfs_traversal(
                    pool.generate((x, y)),
                    EightConnectedExpander::new(&map, &pool, state),
                    |node| {
                        if grid[node.get(state)] == usize::MAX {
                            grid[node.get(state)] = array.len();
                            array.push(node.get(state));
                            true
                        } else {
                            false
                        }
                    },
                );
            }
        }

        GridMapper {
            grid,
            array: array.into_boxed_slice(),
        }
    }

    pub fn load(from: &mut impl Read) -> std::io::Result<Self> {
        let mut bytes = [0; 4];
        from.read_exact(&mut bytes)?;
        let len = u32::from_le_bytes(bytes) as usize;

        from.read_exact(&mut bytes)?;
        let width = i32::from_le_bytes(bytes);
        from.read_exact(&mut bytes)?;
        let height = i32::from_le_bytes(bytes);

        let mut grid = Grid::new(width, height, |_, _| usize::MAX);
        let mut array = vec![(0, 0); len].into_boxed_slice();
        for id in 0..len {
            from.read_exact(&mut bytes)?;
            let x = i32::from_le_bytes(bytes);
            from.read_exact(&mut bytes)?;
            let y = i32::from_le_bytes(bytes);
            grid[(x, y)] = id;
            array[id] = (x, y);
        }

        Ok(GridMapper { grid, array })
    }

    pub fn save(&self, to: &mut impl Write) -> std::io::Result<()> {
        to.write_all(&(self.array.len() as u32).to_le_bytes())?;
        to.write_all(&self.grid.width().to_le_bytes())?;
        to.write_all(&self.grid.height().to_le_bytes())?;
        for (x, y) in self.array.iter() {
            to.write_all(&x.to_le_bytes())?;
            to.write_all(&y.to_le_bytes())?;
        }
        Ok(())
    }
}

impl StateIdMapper for GridMapper {
    type State = (i32, i32);

    fn num_ids(&self) -> usize {
        self.array.len()
    }

    fn state_to_id(&self, state: Self::State) -> usize {
        self.grid[state]
    }

    fn id_to_state(&self, id: usize) -> Self::State {
        self.array[id]
    }
}
