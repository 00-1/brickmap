//! Cellular-automata voxels (E5) — "voxels that behave". Pure logic over a
//! [`Section`] grid; the app steps active sections on a tick and re-meshes the dirty
//! ones (off-thread, M6). Knows only `world` types, nothing about wgpu.
//!
//! v1: **falling sand**. Sand falls straight down into air, else slides diagonally
//! down, so dropped sand settles into plausible piles. Deterministic (a cell parity
//! breaks the left/right tie) so it's reproducible for tests + golden images.

use crate::world::{BlockId, Section};

/// Sand reuses the terrain's sand material (4) — it already looks the part.
pub const SAND: BlockId = BlockId(4);

/// Advance falling sand by one step. Returns `true` if any grain moved (the section is
/// then "dirty" and needs re-meshing). Processes bottom-up so a grain falls at most one
/// cell per step (a stack falls as a column, one cell/tick).
pub fn step_sand(s: &mut Section) -> bool {
    let n = Section::SIZE;
    let mut moved = false;
    for y in 1..n {
        for z in 0..n {
            for x in 0..n {
                if s.get(x, y, z) != SAND {
                    continue;
                }
                // Straight down.
                if s.get(x, y - 1, z).is_air() {
                    s.set(x, y, z, BlockId::AIR);
                    s.set(x, y - 1, z, SAND);
                    moved = true;
                    continue;
                }
                // Diagonally down. Try the four lower-diagonal cells; a per-cell parity
                // flips the x/z order so piles don't all lean one way.
                let flip = (x + z + y) & 1 == 0;
                let mut diag = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)];
                if flip {
                    diag.reverse();
                }
                for (dx, dz) in diag {
                    let (nx, nz) = (x as i32 + dx, z as i32 + dz);
                    if nx < 0 || nz < 0 || nx >= n as i32 || nz >= n as i32 {
                        continue;
                    }
                    let (nx, nz) = (nx as u32, nz as u32);
                    if s.get(nx, y - 1, nz).is_air() {
                        s.set(x, y, z, BlockId::AIR);
                        s.set(nx, y - 1, nz, SAND);
                        moved = true;
                        break;
                    }
                }
            }
        }
    }
    moved
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_sand(s: &Section) -> usize {
        let n = Section::SIZE;
        let mut c = 0;
        for z in 0..n {
            for y in 0..n {
                for x in 0..n {
                    if s.get(x, y, z) == SAND {
                        c += 1;
                    }
                }
            }
        }
        c
    }

    /// The highest occupied y, or `None` if empty.
    fn top(s: &Section) -> Option<u32> {
        let n = Section::SIZE;
        (0..n)
            .rev()
            .find(|&y| (0..n).any(|z| (0..n).any(|x| s.get(x, y, z) == SAND)))
    }

    #[test]
    fn a_floating_grain_falls_to_the_floor() {
        let mut s = Section::new();
        s.set(10, 20, 10, SAND);
        for _ in 0..40 {
            step_sand(&mut s);
        }
        assert_eq!(s.get(10, 0, 10), SAND, "grain should rest on the floor");
        assert!(
            s.get(10, 20, 10).is_air(),
            "grain should have left its start"
        );
        assert_eq!(count_sand(&s), 1, "no grains created or destroyed");
    }

    #[test]
    fn one_step_moves_a_grain_exactly_one_cell() {
        let mut s = Section::new();
        s.set(5, 10, 5, SAND);
        assert!(step_sand(&mut s));
        assert_eq!(s.get(5, 9, 5), SAND);
        assert!(s.get(5, 10, 5).is_air());
    }

    #[test]
    fn sand_is_conserved_and_settles() {
        // A 1-wide column of sand above the floor must settle and stop moving, with the
        // same number of grains. (Diagonal sliding may widen it into a small pile.)
        let mut s = Section::new();
        for y in 1..16 {
            s.set(16, y, 16, SAND);
        }
        let start = count_sand(&s);
        let mut steps = 0;
        while step_sand(&mut s) && steps < 1000 {
            steps += 1;
        }
        assert!(steps < 1000, "sand should reach a resting state");
        assert_eq!(count_sand(&s), start, "sand conserved");
        // It settled onto the floor.
        assert!(top(&s).unwrap_or(99) < 16);
    }

    #[test]
    fn a_pile_spreads_wider_than_one_column() {
        // Drop many grains down a single column; diagonal sliding should spread the
        // base wider than 1×1 rather than stacking a 1-wide tower.
        let mut s = Section::new();
        for _ in 0..30 {
            // keep refilling the top of the column as grains fall away
            if s.get(16, 30, 16).is_air() {
                s.set(16, 30, 16, SAND);
            }
            step_sand(&mut s);
        }
        for _ in 0..400 {
            step_sand(&mut s);
        }
        // Count the footprint on the floor (y == 0).
        let n = Section::SIZE;
        let mut footprint = 0;
        for z in 0..n {
            for x in 0..n {
                if s.get(x, 0, z) == SAND {
                    footprint += 1;
                }
            }
        }
        assert!(footprint > 1, "pile should spread, not stack 1-wide");
    }

    #[test]
    fn resting_sand_reports_not_dirty() {
        // Sand already on the floor with support shouldn't move.
        let mut s = Section::new();
        s.set(4, 0, 4, SAND);
        assert!(!step_sand(&mut s), "grounded grain shouldn't move");
    }
}
