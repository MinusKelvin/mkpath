use std::io::{Read, Write};
use std::sync::Mutex;
use std::time::Duration;

use enumset::EnumSet;
use mkpath_grid::{BitGrid, Direction, Grid};
use mkpath_jps::JumpDatabase;

use crate::first_move::FirstMoveComputer;
use crate::tiebreak::compute_tiebreak_table;
use crate::{independent_jump_points, parallel_for};

pub struct PartialCellBb {
    jump_db: JumpDatabase,
    partial_bb: Grid<Option<[Rectangle; 8]>>,
}

struct Rectangle {
    low_x: i16,
    low_y: i16,
    high_x: i16,
    high_y: i16,
}

impl PartialCellBb {
    pub fn compute(
        map: BitGrid,
        progress_callback: impl FnMut(usize, usize, Duration) + Send,
    ) -> Self {
        // note: this checks that valid coordinates are inside i16 range
        let jump_db = JumpDatabase::new(map);
        let map = jump_db.map();
        let jump_points = independent_jump_points(&jump_db);

        let start = std::time::Instant::now();
        let num_jps = jump_points.len();
        let progress = Mutex::new((0, progress_callback));

        let partial_bb = Mutex::new(Grid::new(
            jump_db.map().width(),
            jump_db.map().height(),
            |_, _| None,
        ));

        parallel_for(
            jump_points.into_iter(),
            || FirstMoveComputer::new(map),
            |fm_computer, (source, jps)| {
                let tiebreak_table =
                    compute_tiebreak_table(map.get_neighborhood(source.0, source.1), jps);

                let mut result = [(); 8].map(|_| Rectangle::empty());

                fm_computer.compute(source, |(x, y), fm| {
                    let fm = tiebreak_table[fm.as_usize()];
                    let best = fm
                        .iter()
                        .min_by_key(|&d| {
                            result[d as usize].area_increase_from_grow(x as i16, y as i16)
                        })
                        .unwrap();
                    result[best as usize].grow(x as i16, y as i16);
                });

                let mut progress = progress.lock().unwrap();
                let (progress, callback) = &mut *progress;
                *progress += 1;
                callback(*progress, num_jps, start.elapsed());

                partial_bb.lock().unwrap()[source] = Some(result);
                Ok(())
            },
        )
        .unwrap();

        PartialCellBb {
            jump_db,
            partial_bb: partial_bb.into_inner().unwrap(),
        }
    }

    pub fn load(map: BitGrid, from: &mut impl Read) -> std::io::Result<Self> {
        let jump_db = JumpDatabase::new(map);

        let mut bytes = [0; 4];
        from.read_exact(&mut bytes)?;
        let num_jps = u32::from_le_bytes(bytes) as usize;

        let mut bytes = [0; 2];
        let mut read_i16 = || from.read(&mut bytes).map(|_| i16::from_le_bytes(bytes));

        let mut partial_bb = Grid::new(jump_db.map().width(), jump_db.map().height(), |_, _| None);
        for _ in 0..num_jps {
            let x = read_i16()? as i32;
            let y = read_i16()? as i32;

            assert!(x >= 0);
            assert!(y >= 0);
            assert!(x < jump_db.map().width());
            assert!(y < jump_db.map().height());

            let mut result = [(); 8].map(|_| Rectangle::empty());
            for dir in 0..8 {
                result[dir] = Rectangle {
                    low_x: read_i16()?,
                    low_y: read_i16()?,
                    high_x: read_i16()?,
                    high_y: read_i16()?,
                }
            }
            partial_bb[(x, y)] = Some(result);
        }

        Ok(PartialCellBb {
            jump_db,
            partial_bb,
        })
    }

    pub fn save(&self, to: &mut impl Write) -> std::io::Result<()> {
        let num = self
            .partial_bb
            .storage()
            .iter()
            .filter(|rects| rects.iter().any(|r| !r.is_empty()))
            .count();
        to.write_all(&u32::to_le_bytes(num as u32))?;
        for y in 0..self.partial_bb.height() {
            for x in 0..self.partial_bb.width() {
                let Some(rects) = &self.partial_bb[(x, y)] else {
                    continue;
                };
                to.write_all(&(x as i16).to_le_bytes())?;
                to.write_all(&(y as i16).to_le_bytes())?;
                for rect in rects {
                    to.write_all(&rect.low_x.to_le_bytes())?;
                    to.write_all(&rect.low_y.to_le_bytes())?;
                    to.write_all(&rect.high_x.to_le_bytes())?;
                    to.write_all(&rect.high_y.to_le_bytes())?;
                }
            }
        }
        Ok(())
    }

    pub fn filter(
        &self,
        pos: (i32, i32),
        target: (i32, i32),
        mut canonical: EnumSet<Direction>,
    ) -> EnumSet<Direction> {
        let Some(rects) = &self.partial_bb[pos] else {
            return canonical;
        };
        for d in canonical {
            if !rects[d as usize].contains(target.0, target.1) {
                canonical.remove(d);
            }
        }
        canonical
    }

    pub fn map(&self) -> &BitGrid {
        self.jump_db.map()
    }

    pub fn jump_db(&self) -> &JumpDatabase {
        &self.jump_db
    }
}

impl Rectangle {
    fn empty() -> Self {
        Rectangle {
            low_x: 0,
            low_y: 0,
            high_x: 0,
            high_y: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.low_x == self.high_x && self.low_y == self.high_y
    }

    fn grow(&mut self, x: i16, y: i16) {
        if self.is_empty() {
            self.low_x = x;
            self.low_y = y;
            self.high_x = x + 1;
            self.high_y = y + 1;
        } else {
            self.low_x = self.low_x.min(x);
            self.low_y = self.low_y.min(y);
            self.high_x = self.high_x.max(x + 1);
            self.high_y = self.high_y.max(y + 1);
        }
    }

    fn area_increase_from_grow(&self, x: i16, y: i16) -> i32 {
        if self.is_empty() {
            return 1;
        }
        let growth_x = (self.low_x - x).max(x - self.high_x).max(0) as i32;
        let growth_y = (self.low_y - y).max(y - self.high_y).max(0) as i32;
        growth_x * (self.high_y - self.low_y) as i32
            + growth_y * (self.high_x - self.low_x) as i32
            + growth_x * growth_y
    }

    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.low_x as i32
            && y >= self.low_y as i32
            && x < self.high_x as i32
            && y < self.high_y as i32
    }
}
