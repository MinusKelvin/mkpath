use std::io::{Read, Write};
use std::sync::Mutex;
use std::time::Duration;

use ahash::HashMap;
use mkpath_cpd::{CpdRow, StateIdMapper};
use mkpath_grid::{BitGrid, Direction};
use mkpath_jps::JumpDatabase;
use rayon::prelude::*;

use crate::first_move::FirstMoveComputer;
use crate::tiebreak::compute_tiebreak_table;
use crate::{independent_jump_points, GridMapper};

pub struct PartialCellCpd {
    mapper: GridMapper,
    jump_db: JumpDatabase,
    partial_cpd: HashMap<(i32, i32), CpdRow>,
}

impl PartialCellCpd {
    pub fn compute(
        map: BitGrid,
        progress_callback: impl FnMut(usize, usize, Duration) + Send,
    ) -> Self {
        let jump_db = JumpDatabase::new(map);
        let map = jump_db.map();
        let mapper = GridMapper::dfs_preorder(map);
        let jump_points = independent_jump_points(map, &jump_db);

        let start = std::time::Instant::now();
        let num_jps = jump_points.len();
        let progress = Mutex::new((0, progress_callback));

        let partial_cpd: HashMap<_, _> = jump_points
            .par_iter()
            .map_init(
                || FirstMoveComputer::new(map, &mapper),
                |fm_computer, (&source, &jps)| {
                    let first_moves = fm_computer.compute(source);

                    let tiebreak_table =
                        compute_tiebreak_table(map.get_neighborhood(source.0, source.1), jps);

                    let result = CpdRow::compress(
                        first_moves
                            .into_iter()
                            .map(|set| tiebreak_table[set.as_usize()].as_u64()),
                    );

                    let mut progress = progress.lock().unwrap();
                    let (progress, callback) = &mut *progress;
                    *progress += 1;
                    callback(*progress, num_jps, start.elapsed());

                    (source, result)
                },
            )
            .collect();

        PartialCellCpd {
            mapper,
            jump_db,
            partial_cpd,
        }
    }

    pub fn load(map: BitGrid, from: &mut impl Read) -> std::io::Result<Self> {
        let jump_db = JumpDatabase::new(map);
        let mapper = GridMapper::load(from)?;

        let mut bytes = [0; 4];
        from.read_exact(&mut bytes)?;
        let num_jps = u32::from_le_bytes(bytes) as usize;

        let mut partial_cpd = HashMap::default();
        for _ in 0..num_jps {
            from.read_exact(&mut bytes)?;
            let x = i32::from_le_bytes(bytes);
            from.read_exact(&mut bytes)?;
            let y = i32::from_le_bytes(bytes);

            assert!(x >= 0);
            assert!(y >= 0);
            assert!(x < jump_db.map().width());
            assert!(y < jump_db.map().height());

            partial_cpd.insert((x, y), CpdRow::load(from)?);
        }

        Ok(PartialCellCpd {
            mapper,
            jump_db,
            partial_cpd,
        })
    }

    pub fn save(&self, to: &mut impl Write) -> std::io::Result<()> {
        self.mapper.save(to)?;
        to.write_all(&u32::to_le_bytes(self.partial_cpd.len() as u32))?;
        for ((x, y), row) in &self.partial_cpd {
            to.write_all(&x.to_le_bytes())?;
            to.write_all(&y.to_le_bytes())?;
            row.save(to)?;
        }
        Ok(())
    }

    pub fn query(&self, pos: (i32, i32), target: (i32, i32)) -> Option<Direction> {
        self.partial_cpd
            .get(&pos)
            .and_then(|row| row.lookup(self.mapper.state_to_id(target)).try_into().ok())
    }

    pub fn map(&self) -> &BitGrid {
        self.jump_db.map()
    }

    pub fn jump_db(&self) -> &JumpDatabase {
        &self.jump_db
    }
}
