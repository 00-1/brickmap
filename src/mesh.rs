//! Turning voxels into geometry — the `mesh` layer from `docs/architecture.md`.
//!
//! M1 uses a **naïve** mesher: for every solid voxel, emit a quad on each face
//! that borders air (or the section boundary). No greedy merging yet — that's M2,
//! and this naïve version becomes the correctness oracle the greedy mesher is
//! tested against. This module is pure CPU work: it knows `world` types but
//! nothing about wgpu. Its output, [`ChunkMesh`], is the contract the renderer
//! consumes.

use bytemuck::{Pod, Zeroable};

use crate::world::{BlockId, Section};

/// One mesh vertex. **Unpacked on purpose** for M1 (easy to read and debug);
/// M2 replaces this with the ≤8-byte packed face vertex from design §9–10.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct ChunkVertex {
    /// Chunk-local position, in voxel units (`0..=SIZE`).
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
}

/// Axis-aligned bounding box of a mesh, in chunk-local space. Used later for
/// frustum culling (M2); here it just travels with the mesh.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Aabb {
    fn of(vertices: &[ChunkVertex]) -> Aabb {
        let Some(first) = vertices.first() else {
            return Aabb {
                min: [0.0; 3],
                max: [0.0; 3],
            };
        };
        let mut min = first.position;
        let mut max = first.position;
        for v in vertices {
            for axis in 0..3 {
                min[axis] = min[axis].min(v.position[axis]);
                max[axis] = max[axis].max(v.position[axis]);
            }
        }
        Aabb { min, max }
    }
}

/// CPU-side mesh for one section: the renderer uploads these buffers directly.
#[derive(Clone, Debug, Default)]
pub struct ChunkMesh {
    pub vertices: Vec<ChunkVertex>,
    pub indices: Vec<u32>,
    pub aabb: Aabb,
}

impl Default for Aabb {
    fn default() -> Self {
        Aabb {
            min: [0.0; 3],
            max: [0.0; 3],
        }
    }
}

impl ChunkMesh {
    /// Number of quads (faces) in the mesh — handy for tests and stats.
    pub fn quad_count(&self) -> usize {
        self.vertices.len() / 4
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// One of the six axis-aligned cube faces: which way it points, and the four
/// corner offsets (0/1 per axis) of its quad relative to a voxel's min corner.
struct Face {
    /// Neighbour direction to test for visibility.
    dir: [i32; 3],
    normal: [f32; 3],
    /// Corners in a consistent order; two triangles are (0,1,2) and (0,2,3).
    corners: [[u32; 3]; 4],
}

#[rustfmt::skip]
const FACES: [Face; 6] = [
    Face { dir: [ 1, 0, 0], normal: [ 1.0, 0.0, 0.0], corners: [[1,0,0],[1,1,0],[1,1,1],[1,0,1]] },
    Face { dir: [-1, 0, 0], normal: [-1.0, 0.0, 0.0], corners: [[0,0,1],[0,1,1],[0,1,0],[0,0,0]] },
    Face { dir: [ 0, 1, 0], normal: [ 0.0, 1.0, 0.0], corners: [[0,1,0],[0,1,1],[1,1,1],[1,1,0]] },
    Face { dir: [ 0,-1, 0], normal: [ 0.0,-1.0, 0.0], corners: [[0,0,1],[0,0,0],[1,0,0],[1,0,1]] },
    Face { dir: [ 0, 0, 1], normal: [ 0.0, 0.0, 1.0], corners: [[0,0,1],[1,0,1],[1,1,1],[0,1,1]] },
    Face { dir: [ 0, 0,-1], normal: [ 0.0, 0.0,-1.0], corners: [[1,0,0],[0,0,0],[0,1,0],[1,1,0]] },
];

/// A face is visible when the neighbouring voxel in `dir` is air, or lies outside
/// the section. M1 meshes a single chunk, so out-of-bounds counts as air (the
/// boundary faces are drawn). M2 supplies neighbour sections to cull these too.
fn neighbour_is_air(section: &Section, x: u32, y: u32, z: u32, dir: [i32; 3]) -> bool {
    let n = Section::SIZE as i32;
    let (nx, ny, nz) = (x as i32 + dir[0], y as i32 + dir[1], z as i32 + dir[2]);
    if nx < 0 || ny < 0 || nz < 0 || nx >= n || ny >= n || nz >= n {
        return true;
    }
    section.get(nx as u32, ny as u32, nz as u32).is_air()
}

/// A simple debug palette so blocks are distinguishable before real materials
/// (textures arrive in M4). Unknown ids render magenta so mistakes are loud.
fn block_color(block: BlockId) -> [f32; 3] {
    match block.0 {
        1 => [0.55, 0.55, 0.58], // stone
        2 => [0.55, 0.40, 0.25], // dirt
        3 => [0.40, 0.70, 0.35], // grass
        4 => [0.80, 0.78, 0.65], // sand
        _ => [0.95, 0.10, 0.95], // unknown
    }
}

/// Mesh one section into a [`ChunkMesh`] (naïve face culling — see module docs).
pub fn mesh_section(section: &Section) -> ChunkMesh {
    let mut vertices: Vec<ChunkVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for z in 0..Section::SIZE {
        for y in 0..Section::SIZE {
            for x in 0..Section::SIZE {
                let block = section.get(x, y, z);
                if block.is_air() {
                    continue;
                }
                let color = block_color(block);

                for face in &FACES {
                    if !neighbour_is_air(section, x, y, z, face.dir) {
                        continue;
                    }
                    let base = vertices.len() as u32;
                    for corner in face.corners {
                        vertices.push(ChunkVertex {
                            position: [
                                (x + corner[0]) as f32,
                                (y + corner[1]) as f32,
                                (z + corner[2]) as f32,
                            ],
                            normal: face.normal,
                            color,
                        });
                    }
                    indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 2,
                        base,
                        base + 2,
                        base + 3,
                    ]);
                }
            }
        }
    }

    let aabb = Aabb::of(&vertices);
    ChunkMesh {
        vertices,
        indices,
        aabb,
    }
}

/// Greedy mesher: merge coplanar, same-material exposed faces into maximal quads
/// (design §7.1). Correctness-first rectangle merging; the bitwise "binary" form
/// is a later optimisation. Produces the *same set of faces* as [`mesh_section`]
/// (the oracle), with far fewer quads.
///
/// Single-section for now: out-of-bounds neighbours count as air (so boundary
/// faces are emitted). Neighbour-aware seam culling is a separate M2 step.
pub fn greedy_mesh_section(section: &Section) -> ChunkMesh {
    let n = Section::SIZE as i32;
    let mut vertices: Vec<ChunkVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // For each principal axis `d` and each facing direction `sign`, sweep slices
    // and greedily merge the plane of exposed faces.
    for d in 0..3usize {
        let u = (d + 1) % 3;
        let v = (d + 2) % 3;

        for sign in [1i32, -1i32] {
            for s in 0..n {
                // mask[i + j*n] = Some(block) where this slice has a face exposed
                // in direction `sign` (solid here, air in the neighbour cell).
                let mut mask: Vec<Option<BlockId>> = vec![None; (n * n) as usize];
                for j in 0..n {
                    for i in 0..n {
                        let mut c = [0i32; 3];
                        c[d] = s;
                        c[u] = i;
                        c[v] = j;
                        let block = section.get(c[0] as u32, c[1] as u32, c[2] as u32);
                        if block.is_air() {
                            continue;
                        }
                        let nd = s + sign;
                        let neighbour_air = nd < 0 || nd >= n || {
                            let mut nb = c;
                            nb[d] = nd;
                            section
                                .get(nb[0] as u32, nb[1] as u32, nb[2] as u32)
                                .is_air()
                        };
                        if neighbour_air {
                            mask[(i + j * n) as usize] = Some(block);
                        }
                    }
                }

                // Greedily merge the mask into maximal rectangles.
                for j in 0..n {
                    let mut i = 0;
                    while i < n {
                        let Some(block) = mask[(i + j * n) as usize] else {
                            i += 1;
                            continue;
                        };
                        // Extend width along u, then height along v.
                        let mut w = 1;
                        while i + w < n && mask[((i + w) + j * n) as usize] == Some(block) {
                            w += 1;
                        }
                        let mut h = 1;
                        'grow: while j + h < n {
                            for k in 0..w {
                                if mask[((i + k) + (j + h) * n) as usize] != Some(block) {
                                    break 'grow;
                                }
                            }
                            h += 1;
                        }

                        emit_quad(
                            &mut vertices,
                            &mut indices,
                            d,
                            u,
                            v,
                            sign,
                            s,
                            i,
                            j,
                            w,
                            h,
                            block,
                        );

                        for hh in 0..h {
                            for ww in 0..w {
                                mask[((i + ww) + (j + hh) * n) as usize] = None;
                            }
                        }
                        i += w;
                    }
                }
            }
        }
    }

    let aabb = Aabb::of(&vertices);
    ChunkMesh {
        vertices,
        indices,
        aabb,
    }
}

/// Emit one greedy quad: a `w`×`h` rectangle on axis `d`'s slice `s`, facing
/// `sign`, spanning `[i, i+w]` along `u` and `[j, j+h]` along `v`.
#[allow(clippy::too_many_arguments)]
fn emit_quad(
    vertices: &mut Vec<ChunkVertex>,
    indices: &mut Vec<u32>,
    d: usize,
    u: usize,
    v: usize,
    sign: i32,
    s: i32,
    i: i32,
    j: i32,
    w: i32,
    h: i32,
    block: BlockId,
) {
    // The face plane sits at the +d boundary of the slice for +sign, else at -d.
    let plane = if sign > 0 { s + 1 } else { s };
    let corner = |uu: i32, vv: i32| -> [f32; 3] {
        let mut p = [0.0f32; 3];
        p[d] = plane as f32;
        p[u] = uu as f32;
        p[v] = vv as f32;
        p
    };
    let mut normal = [0.0f32; 3];
    normal[d] = sign as f32;
    let color = block_color(block);

    let (a, b, c, e) = (
        corner(i, j),
        corner(i + w, j),
        corner(i + w, j + h),
        corner(i, j + h),
    );
    // Wind so the front face matches the normal direction.
    let quad = if sign > 0 { [a, b, c, e] } else { [a, e, c, b] };

    let base = vertices.len() as u32;
    for position in quad {
        vertices.push(ChunkVertex {
            position,
            normal,
            color,
        });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

#[cfg(test)]
mod tests {
    use super::*;

    const STONE: BlockId = BlockId(1);

    fn section_with(solids: &[(u32, u32, u32)]) -> Section {
        let mut s = Section::new();
        for &(x, y, z) in solids {
            s.set(x, y, z, STONE);
        }
        s
    }

    #[test]
    fn empty_section_meshes_to_nothing() {
        let mesh = mesh_section(&Section::new());
        assert!(mesh.is_empty());
        assert_eq!(mesh.quad_count(), 0);
        assert!(mesh.vertices.is_empty());
    }

    #[test]
    fn single_voxel_has_six_faces() {
        let mesh = mesh_section(&section_with(&[(10, 10, 10)]));
        assert_eq!(mesh.quad_count(), 6);
        assert_eq!(mesh.vertices.len(), 24);
        assert_eq!(mesh.indices.len(), 36);
    }

    #[test]
    fn two_adjacent_voxels_cull_the_shared_face() {
        // 6 + 6 faces minus the 2 that touch each other.
        let mesh = mesh_section(&section_with(&[(10, 10, 10), (11, 10, 10)]));
        assert_eq!(mesh.quad_count(), 10);
    }

    #[test]
    fn solid_two_cubed_block_shows_only_outer_faces() {
        let mut solids = Vec::new();
        for z in 10..12 {
            for y in 10..12 {
                for x in 10..12 {
                    solids.push((x, y, z));
                }
            }
        }
        // Each of the 8 voxels exposes exactly 3 faces -> 24 outer faces.
        let mesh = mesh_section(&section_with(&solids));
        assert_eq!(mesh.quad_count(), 24);
    }

    #[test]
    fn fully_solid_section_meshes_only_its_surface() {
        let mut s = Section::new();
        for z in 0..Section::SIZE {
            for y in 0..Section::SIZE {
                for x in 0..Section::SIZE {
                    s.set(x, y, z, STONE);
                }
            }
        }
        // Six faces of a 32x32 sheet; the whole interior is culled.
        let expected = 6 * Section::SIZE as usize * Section::SIZE as usize;
        let mesh = mesh_section(&s);
        assert_eq!(mesh.quad_count(), expected);
    }

    #[test]
    fn indices_are_valid_and_triangulated() {
        let mesh = mesh_section(&section_with(&[(0, 0, 0), (5, 9, 17), (31, 31, 31)]));
        assert_eq!(mesh.indices.len() % 3, 0);
        let vcount = mesh.vertices.len() as u32;
        assert!(mesh.indices.iter().all(|&i| i < vcount));
        assert_eq!(mesh.vertices.len(), mesh.quad_count() * 4);
        assert_eq!(mesh.indices.len(), mesh.quad_count() * 6);
    }

    #[test]
    fn boundary_voxel_still_emits_its_outer_faces() {
        // A voxel in the corner: 3 neighbours are out of bounds (treated as air),
        // 3 are in-bounds air -> all 6 faces present.
        let mesh = mesh_section(&section_with(&[(0, 0, 0)]));
        assert_eq!(mesh.quad_count(), 6);
    }

    #[test]
    fn aabb_bounds_a_single_voxel() {
        let mesh = mesh_section(&section_with(&[(10, 11, 12)]));
        assert_eq!(mesh.aabb.min, [10.0, 11.0, 12.0]);
        assert_eq!(mesh.aabb.max, [11.0, 12.0, 13.0]);
    }

    // --- Greedy mesher --------------------------------------------------------

    /// Decompose a mesh into its set of unit cell-faces, keyed by
    /// (axis, sign, plane, u-cell, v-cell, material). Greedy and naïve meshes must
    /// produce the *same* set; greedy just packs them into fewer quads.
    fn face_cells(mesh: &ChunkMesh) -> Vec<(usize, i32, i32, i32, i32, i32)> {
        let mut cells = Vec::new();
        for q in 0..mesh.quad_count() {
            let base = q * 4;
            let normal = mesh.vertices[base].normal;
            let a = (0..3).find(|&k| normal[k].abs() > 0.5).unwrap();
            let sign = if normal[a] > 0.0 { 1 } else { -1 };
            let (u, v) = ((a + 1) % 3, (a + 2) % 3);

            let pos: Vec<[f32; 3]> = (0..4).map(|k| mesh.vertices[base + k].position).collect();
            let plane = pos[0][a] as i32;
            let umin = pos.iter().map(|p| p[u] as i32).min().unwrap();
            let umax = pos.iter().map(|p| p[u] as i32).max().unwrap();
            let vmin = pos.iter().map(|p| p[v] as i32).min().unwrap();
            let vmax = pos.iter().map(|p| p[v] as i32).max().unwrap();

            let c = mesh.vertices[base].color;
            let mat = (c[0] * 255.0).round() as i32 * 1_000_000
                + (c[1] * 255.0).round() as i32 * 1000
                + (c[2] * 255.0).round() as i32;

            for uu in umin..umax {
                for vv in vmin..vmax {
                    cells.push((a, sign, plane, uu, vv, mat));
                }
            }
        }
        cells
    }

    /// The same scenes the naïve mesher is tested on, plus a multi-material and a
    /// chunk-border case — greedy must match the oracle on all of them.
    fn oracle_fixtures() -> Vec<Section> {
        let mut multi = section_with(&[(4, 4, 4), (5, 4, 4)]);
        multi.set(5, 4, 4, BlockId(2)); // a different material next to a STONE
        let mut full = Section::new();
        for z in 0..Section::SIZE {
            for y in 0..Section::SIZE {
                for x in 0..Section::SIZE {
                    full.set(x, y, z, STONE);
                }
            }
        }
        vec![
            Section::new(),
            section_with(&[(10, 10, 10)]),
            section_with(&[(10, 10, 10), (11, 10, 10)]),
            section_with(&[(0, 0, 0), (31, 31, 31)]), // chunk corners
            multi,
            full,
        ]
    }

    #[test]
    fn greedy_covers_exactly_the_same_faces_as_naive() {
        use std::collections::HashSet;
        for (n, section) in oracle_fixtures().iter().enumerate() {
            let mut naive = face_cells(&mesh_section(section));
            let mut greedy = face_cells(&greedy_mesh_section(section));

            // Greedy must not emit a unit face twice (no overlapping quads).
            let unique: HashSet<_> = greedy.iter().copied().collect();
            assert_eq!(unique.len(), greedy.len(), "fixture {n}: greedy overlaps");

            naive.sort_unstable();
            greedy.sort_unstable();
            assert_eq!(naive, greedy, "fixture {n}: greedy != naive face set");
        }
    }

    #[test]
    fn greedy_merges_a_solid_box_into_six_quads() {
        // Any solid, uniform-material rectangular box has 6 faces, each one quad.
        let mut full = Section::new();
        for z in 0..Section::SIZE {
            for y in 0..Section::SIZE {
                for x in 0..Section::SIZE {
                    full.set(x, y, z, STONE);
                }
            }
        }
        let mesh = greedy_mesh_section(&full);
        assert_eq!(mesh.quad_count(), 6);
        // ...versus the naïve mesher's per-cell surface.
        assert_eq!(mesh_section(&full).quad_count(), 6 * 32 * 32);
    }

    #[test]
    fn greedy_merges_a_flat_slab_layer() {
        // One full y-layer (32x32x1): top + bottom + four edge strips = 6 quads.
        let mut slab = Section::new();
        for z in 0..Section::SIZE {
            for x in 0..Section::SIZE {
                slab.set(x, 0, z, STONE);
            }
        }
        assert_eq!(greedy_mesh_section(&slab).quad_count(), 6);
    }

    #[test]
    fn greedy_indices_are_valid_and_triangulated() {
        let mesh = greedy_mesh_section(&section_with(&[(0, 0, 0), (5, 9, 17), (31, 31, 31)]));
        assert_eq!(mesh.indices.len() % 3, 0);
        let vcount = mesh.vertices.len() as u32;
        assert!(mesh.indices.iter().all(|&i| i < vcount));
        assert_eq!(mesh.vertices.len(), mesh.quad_count() * 4);
    }
}
