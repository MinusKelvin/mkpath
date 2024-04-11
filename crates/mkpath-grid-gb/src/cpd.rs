use std::io::{Read, Write};
use std::sync::Mutex;
use std::time::Duration;

use ahash::HashMap;
use enumset::EnumSet;
use mkpath_cpd::{CpdRow, StateIdMapper};
use mkpath_grid::{BitGrid, Direction, Grid};
use mkpath_jps::JumpDatabase;
use rayon::prelude::*;

use crate::first_move::FirstMoveComputer;
use crate::independent_jump_points;
use crate::mapper::GridMapper;
use crate::tiebreak::compute_tiebreak_table;

pub struct PartialCellCpd {
    mapper: GridMapper,
    jump_db: JumpDatabase,
    partial_cpd: Grid<Option<Box<CpdRow>>>,
}

impl PartialCellCpd {
    pub fn compute(
        map: BitGrid,
        mut progress_callback: impl FnMut(usize, usize, Duration) + Send,
    ) -> Self {
        let jump_db = JumpDatabase::new(map);
        let mapper = GridMapper::dfs_preorder(jump_db.map());
        let jump_points = independent_jump_points(&jump_db);
        let mut partial_cpd = Grid::new(jump_db.map().width(), jump_db.map().height(), |_, _| None);
        Self::compute_impl(
            &jump_db,
            &mapper,
            jump_points,
            |progress, total, time, source, result| {
                partial_cpd[source] = Some(result);
                progress_callback(progress, total, time);
                Ok(())
            },
        )
        .unwrap();

        PartialCellCpd {
            mapper,
            jump_db,
            partial_cpd,
        }
    }

    pub fn compute_to_file(
        map: BitGrid,
        to: &mut (impl Write + Send),
        mut progress_callback: impl FnMut(usize, usize, Duration) + Send,
    ) -> std::io::Result<()> {
        let jump_db = JumpDatabase::new(map);
        let mapper = GridMapper::dfs_preorder(jump_db.map());
        let jump_points = independent_jump_points(&jump_db);
        mapper.save(to)?;
        to.write_all(&u32::to_le_bytes(jump_points.len() as u32))?;
        Self::compute_impl(
            &jump_db,
            &mapper,
            jump_points,
            |progress, total, time, (x, y), result| {
                to.write_all(&x.to_le_bytes())?;
                to.write_all(&y.to_le_bytes())?;
                result.save(to)?;
                progress_callback(progress, total, time);
                Ok(())
            },
        )
    }

    fn compute_impl<F>(
        jump_db: &JumpDatabase,
        mapper: &GridMapper,
        jump_points: HashMap<(i32, i32), EnumSet<Direction>>,
        iter_done: F,
    ) -> std::io::Result<()>
    where
        F: FnMut(usize, usize, Duration, (i32, i32), Box<CpdRow>) -> std::io::Result<()> + Send,
    {
        let map = jump_db.map();

        let start = std::time::Instant::now();
        let num_jps = jump_points.len();
        let progress = Mutex::new((0, iter_done));

        jump_points.par_iter().try_for_each_init(
            || FirstMoveComputer::new(map),
            |fm_computer, (&source, &jps)| {
                let mut first_moves = vec![EnumSet::all(); mapper.num_ids()];
                fm_computer.compute(source, |pos, fm| first_moves[mapper.state_to_id(pos)] = fm);

                let tiebreak_table =
                    compute_tiebreak_table(map.get_neighborhood(source.0, source.1), jps);
                let result = CpdRow::compress(
                    first_moves
                        .into_iter()
                        .map(|fm| tiebreak_table[fm.as_usize()].as_u64()),
                );

                let mut progress = progress.lock().unwrap();
                let (progress, callback) = &mut *progress;
                *progress += 1;
                callback(*progress, num_jps, start.elapsed(), source, result)
            },
        )
    }

    pub fn load(map: BitGrid, from: &mut impl Read) -> std::io::Result<Self> {
        let jump_db = JumpDatabase::new(map);
        let mapper = GridMapper::load(from)?;

        let mut bytes = [0; 4];
        from.read_exact(&mut bytes)?;
        let num_jps = u32::from_le_bytes(bytes) as usize;

        let mut partial_cpd = Grid::new(jump_db.map().width(), jump_db.map().height(), |_, _| None);
        for _ in 0..num_jps {
            from.read_exact(&mut bytes)?;
            let x = i32::from_le_bytes(bytes);
            from.read_exact(&mut bytes)?;
            let y = i32::from_le_bytes(bytes);

            assert!(x >= 0);
            assert!(y >= 0);
            assert!(x < jump_db.map().width());
            assert!(y < jump_db.map().height());

            partial_cpd[(x, y)] = Some(CpdRow::load(from)?);
        }

        Ok(PartialCellCpd {
            mapper,
            jump_db,
            partial_cpd,
        })
    }

    pub fn save(&self, to: &mut impl Write) -> std::io::Result<()> {
        self.mapper.save(to)?;
        let num_entries = self
            .partial_cpd
            .storage()
            .iter()
            .filter(|row| row.is_some())
            .count();
        to.write_all(&u32::to_le_bytes(num_entries as u32))?;
        for y in 0..self.partial_cpd.height() {
            for x in 0..self.partial_cpd.width() {
                let Some(row) = &self.partial_cpd[(x, y)] else {
                    continue;
                };
                to.write_all(&x.to_le_bytes())?;
                to.write_all(&y.to_le_bytes())?;
                row.save(to)?;
            }
        }
        Ok(())
    }

    pub fn query(&self, pos: (i32, i32), target: (i32, i32)) -> Option<Direction> {
        self.partial_cpd[pos]
            .as_ref()
            .and_then(|row| row.lookup(self.mapper.state_to_id(target)).try_into().ok())
    }

    pub fn map(&self) -> &BitGrid {
        self.jump_db.map()
    }

    pub fn jump_db(&self) -> &JumpDatabase {
        &self.jump_db
    }
}
