//! Visibility-graph "cave" culling (M5). Two pure pieces:
//!
//! - [`connectivity`] bakes, per section, which of its 6 faces can see each other
//!   *through air* (flood-fill the air cells; two faces connect if their air regions
//!   touch). A [`FaceGraph`].
//! - [`visible_set`] floods the loaded chunks from the camera's chunk, stepping into a
//!   neighbour only through a connected face, so chunks sealed off by solid rock from
//!   the camera are culled. Layered on the frustum test.
//!
//! Faces use the same order as [`crate::mesh::Neighbors`]: `0:-x 1:+x 2:-y 3:+y
//! 4:-z 5:+z`. Pure logic — no wgpu.

use std::collections::{HashSet, VecDeque};

use crate::world::{ChunkCoord, Section};

/// Which of a section's 6 faces connect to each other through air. `rows[f]` has bit
/// `g` set iff faces `f` and `g` share an air region (symmetric; a face with any air
/// connects to itself).
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct FaceGraph {
    rows: [u8; 6],
}

impl FaceGraph {
    /// Every face connected to every face — used for an unloaded (assumed-open) chunk.
    pub fn open() -> FaceGraph {
        FaceGraph {
            rows: [0b11_1111; 6],
        }
    }

    /// Can sight pass from face `a` to face `b` through this section's air?
    pub fn connects(&self, a: usize, b: usize) -> bool {
        self.rows[a] & (1 << b) != 0
    }
}

/// The neighbour chunk across `face` (`0:-x 1:+x 2:-y 3:+y 4:-z 5:+z`).
fn step(coord: ChunkCoord, face: usize) -> ChunkCoord {
    let (x, y, z) = coord;
    match face {
        0 => (x - 1, y, z),
        1 => (x + 1, y, z),
        2 => (x, y - 1, z),
        3 => (x, y + 1, z),
        4 => (x, y, z - 1),
        _ => (x, y, z + 1),
    }
}

/// Build the face-connectivity graph for a section (flood-fill air components).
pub fn connectivity(section: &Section) -> FaceGraph {
    let n = Section::SIZE as usize;
    let idx = |x: usize, y: usize, z: usize| x + y * n + z * n * n;
    let mut comp_of = vec![u16::MAX; n * n * n]; // MAX = solid / unvisited
    let mut rows = [0u8; 6];

    for sz in 0..n {
        for sy in 0..n {
            for sx in 0..n {
                if !section.get(sx as u32, sy as u32, sz as u32).is_air() {
                    continue;
                }
                if comp_of[idx(sx, sy, sz)] != u16::MAX {
                    continue;
                }
                // Flood this air component, accumulating the faces it touches.
                let mut touched = 0u8;
                let mut q = VecDeque::new();
                comp_of[idx(sx, sy, sz)] = 0;
                q.push_back((sx, sy, sz));
                while let Some((x, y, z)) = q.pop_front() {
                    if x == 0 {
                        touched |= 1 << 0;
                    }
                    if x == n - 1 {
                        touched |= 1 << 1;
                    }
                    if y == 0 {
                        touched |= 1 << 2;
                    }
                    if y == n - 1 {
                        touched |= 1 << 3;
                    }
                    if z == 0 {
                        touched |= 1 << 4;
                    }
                    if z == n - 1 {
                        touched |= 1 << 5;
                    }
                    let mut visit = |nx: i32, ny: i32, nz: i32, q: &mut VecDeque<_>| {
                        if nx < 0 || ny < 0 || nz < 0 {
                            return;
                        }
                        let (nx, ny, nz) = (nx as usize, ny as usize, nz as usize);
                        if nx >= n || ny >= n || nz >= n {
                            return;
                        }
                        if !section.get(nx as u32, ny as u32, nz as u32).is_air() {
                            return;
                        }
                        if comp_of[idx(nx, ny, nz)] != u16::MAX {
                            return;
                        }
                        comp_of[idx(nx, ny, nz)] = 0;
                        q.push_back((nx, ny, nz));
                    };
                    let (xi, yi, zi) = (x as i32, y as i32, z as i32);
                    visit(xi - 1, yi, zi, &mut q);
                    visit(xi + 1, yi, zi, &mut q);
                    visit(xi, yi - 1, zi, &mut q);
                    visit(xi, yi + 1, zi, &mut q);
                    visit(xi, yi, zi - 1, &mut q);
                    visit(xi, yi, zi + 1, &mut q);
                }
                // Every pair of faces this component touches is mutually connected.
                for (a, row) in rows.iter_mut().enumerate() {
                    if touched & (1 << a) != 0 {
                        *row |= touched;
                    }
                }
            }
        }
    }
    FaceGraph { rows }
}

/// Flood the loaded chunks from the camera's chunk through connected faces; return the
/// set that is reachable through air **and** in the frustum (plus the camera chunk).
/// `graph_of` returns a loaded chunk's graph (or `None` if not loaded — traversal
/// stops there); `in_frustum` gates which reached chunks are actually visible. Only
/// loaded chunks are traversed, so the search is bounded by the loaded set.
pub fn visible_set(
    camera_chunk: ChunkCoord,
    graph_of: impl Fn(ChunkCoord) -> Option<FaceGraph>,
    in_frustum: impl Fn(ChunkCoord) -> bool,
) -> HashSet<ChunkCoord> {
    let mut visible = HashSet::new();
    let mut visited: HashSet<(ChunkCoord, u8)> = HashSet::new();
    let mut q: VecDeque<(ChunkCoord, u8)> = VecDeque::new();

    // entry face 6 == "you start here": the camera chunk can be exited any way.
    visible.insert(camera_chunk);
    visited.insert((camera_chunk, 6));
    q.push_back((camera_chunk, 6));

    while let Some((coord, entry)) = q.pop_front() {
        let graph = graph_of(coord);
        for exit in 0usize..6 {
            let passable = entry == 6 || graph.is_none_or(|g| g.connects(entry as usize, exit));
            if !passable {
                continue;
            }
            let nbr = step(coord, exit);
            if graph_of(nbr).is_none() {
                continue; // don't wander past the loaded set
            }
            let nentry = (exit ^ 1) as u8;
            if visited.insert((nbr, nentry)) {
                if in_frustum(nbr) {
                    visible.insert(nbr);
                }
                q.push_back((nbr, nentry));
            }
        }
    }
    visible
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::BlockId;

    const STONE: BlockId = BlockId(1);

    fn solid_section() -> Section {
        let mut s = Section::new();
        for z in 0..Section::SIZE {
            for y in 0..Section::SIZE {
                for x in 0..Section::SIZE {
                    s.set(x, y, z, STONE);
                }
            }
        }
        s
    }

    #[test]
    fn empty_section_connects_all_faces() {
        let g = connectivity(&Section::new());
        for a in 0..6 {
            for b in 0..6 {
                assert!(g.connects(a, b), "{a}->{b} should connect in open air");
            }
        }
    }

    #[test]
    fn solid_section_connects_nothing() {
        let g = connectivity(&solid_section());
        for a in 0..6 {
            for b in 0..6 {
                assert!(!g.connects(a, b));
            }
        }
    }

    #[test]
    fn a_solid_wall_splits_the_two_sides() {
        // Fill a solid plane at x = 16, leaving air on both sides.
        let mut s = Section::new();
        let mid = Section::SIZE / 2;
        for z in 0..Section::SIZE {
            for y in 0..Section::SIZE {
                s.set(mid, y, z, STONE);
            }
        }
        let g = connectivity(&s);
        // -x (0) and +x (1) air regions are separated by the wall.
        assert!(!g.connects(0, 1), "wall should block -x..+x");
        // But -y and +y still connect within each side.
        assert!(g.connects(2, 3));
    }

    #[test]
    fn open_scene_keeps_every_in_frustum_chunk() {
        // A 5x5x1 grid of fully-open chunks; camera in the middle, frustum = all.
        let all_open = |_c: ChunkCoord| Some(FaceGraph::open());
        let loaded = |c: ChunkCoord| (-2..=2).contains(&c.0) && c.1 == 0 && (-2..=2).contains(&c.2);
        let graph_of = |c: ChunkCoord| if loaded(c) { all_open(c) } else { None };
        let vis = visible_set((0, 0, 0), graph_of, |_| true);
        // Nothing is culled in an open scene: all 25 loaded chunks are visible.
        let mut count = 0;
        for x in -2..=2 {
            for z in -2..=2 {
                assert!(vis.contains(&(x, 0, z)), "open chunk ({x},0,{z}) culled");
                count += 1;
            }
        }
        assert_eq!(count, 25);
    }

    #[test]
    fn a_chunk_behind_a_seal_is_culled() {
        // Four chunks in a row on +x: camera, open, SEALED (no face connects), open.
        // You can see the sealed chunk's near rock face, but sight can't pass through
        // it — so the open chunk *behind* it is culled, even though it's in frustum.
        let graph_of = |c: ChunkCoord| match c {
            (0, 0, 0) | (1, 0, 0) | (3, 0, 0) => Some(FaceGraph::open()),
            (2, 0, 0) => Some(FaceGraph::default()), // sealed: rows all 0
            _ => None,
        };
        let vis = visible_set((0, 0, 0), graph_of, |_| true);
        assert!(vis.contains(&(1, 0, 0)), "open neighbour should be visible");
        assert!(vis.contains(&(2, 0, 0)), "the seal's near face is visible");
        assert!(
            !vis.contains(&(3, 0, 0)),
            "chunk hidden behind the seal must be culled",
        );
    }
}
