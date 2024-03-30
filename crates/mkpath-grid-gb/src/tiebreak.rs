use enumset::EnumSet;
use mkpath_grid::Direction;
use mkpath_jps::canonical_successors;

pub fn compute_tiebreak_table(
    nb: EnumSet<Direction>,
    jps: EnumSet<Direction>,
) -> [EnumSet<Direction>; 256] {
    let valid_moves = canonical_successors(nb, None);
    let mut result = [EnumSet::empty(); 256];
    // empty first move set is invalid, so skip it and put full wildcard
    result[0] = EnumSet::all();
    for fm in 1..256 {
        let fm_dirs = EnumSet::from_u8(fm as u8);
        result[fm] = fm_dirs;

        if !fm_dirs.is_subset(valid_moves) {
            // first move set is invalid because it contains illegal moves; skip
            continue;
        }

        for jp in jps {
            if is_irrelevant_jp(jp, fm_dirs, nb) {
                continue;
            }
            result[fm] &= canonical_successors(nb, Some(jp));
        }

        assert!(!result[fm].is_empty());
    }
    result
}

fn is_irrelevant_jp(jp: Direction, fm: EnumSet<Direction>, nb: EnumSet<Direction>) -> bool {
    use Direction::*;

    let canonical = canonical_successors(nb, Some(jp));
    // Simple non-optimal/non-canonical case
    if canonical.is_disjoint(fm) {
        return true;
    }

    // Cases 1, 2, 3, and 4 (backwards, switchback, and diagonal-to-diagonal turn)
    if !fm.is_disjoint(match jp {
        North => SouthWest | South | SouthEast,
        West => NorthEast | East | SouthEast,
        South => NorthWest | North | NorthEast,
        East => NorthWest | West | SouthWest,
        NorthWest => SouthWest | South | SouthEast | East | NorthEast,
        SouthWest => SouthEast | East | NorthEast | North | NorthWest,
        SouthEast => NorthEast | North | NorthWest | West | SouthWest,
        NorthEast => NorthWest | West | SouthWest | South | SouthEast,
    }) {
        return true;
    }

    // Case 5 (orthogonal-to-orthogonal turn)
    match jp {
        North if fm.contains(West) && nb.contains(SouthWest) => return true,
        North if fm.contains(East) && nb.contains(SouthEast) => return true,
        West if fm.contains(South) && nb.contains(SouthEast) => return true,
        West if fm.contains(North) && nb.contains(NorthEast) => return true,
        South if fm.contains(West) && nb.contains(NorthWest) => return true,
        South if fm.contains(East) && nb.contains(NorthEast) => return true,
        East if fm.contains(South) && nb.contains(SouthWest) => return true,
        East if fm.contains(North) && nb.contains(NorthWest) => return true,
        _ => {}
    }

    false
}

#[test]
fn tiebreaking_is_valid() {
    use Direction::*;

    for nb in 0..256 {
        let nb = EnumSet::from_u8(nb as u8);

        let mut jp_successors = EnumSet::empty();
        let mut jp = EnumSet::empty();

        // find orthogonal jump points
        for dir in [North, South, East, West] {
            if !nb.contains(dir.backwards()) {
                continue;
            }
            let successors = canonical_successors(nb, Some(dir));
            if successors & dir != successors {
                jp_successors |= successors;
                jp |= dir;
            }
        }

        // find potential diagonal jump points
        for dir in [NorthWest, NorthEast, SouthWest, SouthEast] {
            let dir_x = match dir {
                NorthWest | SouthWest => West,
                _ => East,
            };
            let dir_y = match dir {
                NorthWest | NorthEast => North,
                _ => South,
            };

            if nb.is_superset(dir_x.backwards() | dir_y.backwards() | dir.backwards())
                && !nb.is_disjoint(dir_x | dir_y)
            {
                // possible diagonal jump point
                // note that since more jump points can only make the valid first move set smaller,
                // we don't need to check the same neighborhood with fewer diagonal jump points
                jp |= dir;
                jp_successors |= canonical_successors(nb, Some(dir));
            }
        }

        if jp.is_empty() {
            continue;
        }

        // computation of tiebreak table checks the non-empty invariant
        compute_tiebreak_table(nb, jp);
    }
}
