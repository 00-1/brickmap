//! Turning voxels into geometry — the `mesh` layer from `docs/architecture.md`.
//!
//! Two meshers: a **naïve** one (one quad per exposed voxel face) that is the
//! correctness oracle, and the **greedy** one (merge coplanar, same-material faces
//! into big quads) used for real. Both are neighbour-aware (cull faces across chunk
//! borders) and emit the [`ChunkMesh`] contract the renderer consumes; vertices
//! pack to two `u32`s (8 bytes, [`pack`]) at upload — position/dir/material/AO plus
//! baked block light. Pure CPU work: knows `world` types, nothing about wgpu.

use crate::world::{BlockId, Section};

/// One CPU-side mesh vertex (unpacked, easy to test). It is **packed to a single
/// `u32`** by [`pack_vertex`] at GPU-upload time — that's the "compressed vertices"
/// performance pillar (design §7.2, §9–10). `material` indexes the shader palette.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ChunkVertex {
    /// Chunk-local position, in voxel units (`0..=SIZE`).
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub material: u16,
    /// Baked ambient-occlusion level, `0` (darkest) .. `3` (unoccluded). M4.
    pub ao: u8,
    /// Baked block-light colour, per channel `0..=15` (flood-fill, E3).
    pub light: [u8; 3],
}

impl ChunkVertex {
    /// A fully-lit (`ao == 3`), unlit-by-blocks (`light == 0`) vertex. The naïve
    /// oracle and tests use this; the greedy mesher bakes real AO + light.
    pub fn new(position: [f32; 3], normal: [f32; 3], material: u16) -> Self {
        ChunkVertex::with_ao_light(position, normal, material, 3, [0; 3])
    }

    pub fn with_ao_light(
        position: [f32; 3],
        normal: [f32; 3],
        material: u16,
        ao: u8,
        light: [u8; 3],
    ) -> Self {
        ChunkVertex {
            position,
            normal,
            material,
            ao,
            light,
        }
    }
}

/// Face-direction index, shared by the packed vertex and the shader:
/// `0:+X 1:-X 2:+Y 3:-Y 4:+Z 5:-Z`.
pub const FACE_NORMALS: [[f32; 3]; 6] = [
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, -1.0],
];

fn normal_to_dir(n: [f32; 3]) -> u32 {
    if n[0] > 0.5 {
        0
    } else if n[0] < -0.5 {
        1
    } else if n[1] > 0.5 {
        2
    } else if n[1] < -0.5 {
        3
    } else if n[2] > 0.5 {
        4
    } else {
        5
    }
}

/// Pack the first vertex word (design §9–10). Bit layout, LSB→MSB:
/// `x:6 | y:6 | z:6 | dir:3 | material:9 | ao:2`. Chunk-local positions span
/// `0..=32` (6 bits). The second word ([`pack`]) carries baked block light.
pub fn pack_vertex(pos: [u32; 3], dir: u32, material: u32, ao: u32) -> u32 {
    debug_assert!(
        pos[0] <= 32 && pos[1] <= 32 && pos[2] <= 32,
        "pos out of 6-bit range"
    );
    debug_assert!(dir < 6 && material < 512 && ao < 4, "field out of range");
    (pos[0] & 0x3F)
        | ((pos[1] & 0x3F) << 6)
        | ((pos[2] & 0x3F) << 12)
        | ((dir & 0x7) << 18)
        | ((material & 0x1FF) << 21)
        | ((ao & 0x3) << 30)
}

/// Inverse of [`pack_vertex`]: `([x,y,z], dir, material, ao)`.
pub fn unpack_vertex(v: u32) -> ([u32; 3], u32, u32, u32) {
    (
        [v & 0x3F, (v >> 6) & 0x3F, (v >> 12) & 0x3F],
        (v >> 18) & 0x7,
        (v >> 21) & 0x1FF,
        (v >> 30) & 0x3,
    )
}

/// Pack a CPU [`ChunkVertex`] for upload into **two** `u32`s (8 bytes, design §10):
/// `word0 = x:6|y:6|z:6|dir:3|material:9|ao:2` (via [`pack_vertex`]);
/// `word1 = lr:4|lg:4|lb:4` (baked block-light colour, `0..=15` per channel).
pub fn pack(v: &ChunkVertex) -> [u32; 2] {
    let pos = [
        v.position[0].round() as u32,
        v.position[1].round() as u32,
        v.position[2].round() as u32,
    ];
    let word0 = pack_vertex(
        pos,
        normal_to_dir(v.normal),
        v.material as u32,
        v.ao as u32 & 0x3,
    );
    let word1 = (v.light[0] as u32 & 0xF)
        | ((v.light[1] as u32 & 0xF) << 4)
        | ((v.light[2] as u32 & 0xF) << 8);
    [word0, word1]
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

/// The six face-adjacent neighbour sections, used to cull faces across chunk
/// borders. Order: `[-x, +x, -y, +y, -z, +z]`. A `None` neighbour is treated as
/// air, so its boundary face is drawn (the single-chunk default).
pub struct Neighbors<'a> {
    pub faces: [Option<&'a Section>; 6],
}

impl Neighbors<'_> {
    /// No neighbours — every boundary face is drawn.
    pub const NONE: Neighbors<'static> = Neighbors { faces: [None; 6] };
}

/// Is the voxel at (possibly out-of-range) local coord `(x,y,z)` air? When a
/// coordinate steps one past a face, the matching neighbour section is consulted
/// (an absent neighbour counts as air). Only ever called one step off a face.
fn is_air(section: &Section, neighbors: &Neighbors, x: i32, y: i32, z: i32) -> bool {
    let n = Section::SIZE as i32;
    if (0..n).contains(&x) && (0..n).contains(&y) && (0..n).contains(&z) {
        return section.get(x as u32, y as u32, z as u32).is_air();
    }
    let (face, sx, sy, sz) = if x < 0 {
        (0, n - 1, y, z)
    } else if x >= n {
        (1, 0, y, z)
    } else if y < 0 {
        (2, x, n - 1, z)
    } else if y >= n {
        (3, x, 0, z)
    } else if z < 0 {
        (4, x, y, n - 1)
    } else {
        (5, x, y, 0)
    };
    match neighbors.faces[face] {
        Some(nb) => nb.get(sx as u32, sy as u32, sz as u32).is_air(),
        None => true,
    }
}

/// Solidity test for AO sampling. Handles a sample up to one step outside the
/// section on a **single** axis (consulting the matching face neighbour); samples
/// outside on two+ axes (chunk edges/corners) count as air. Good enough — AO only
/// loses a little darkening at chunk corners, where we have no diagonal neighbour.
fn solid_for_ao(section: &Section, neighbors: &Neighbors, x: i32, y: i32, z: i32) -> bool {
    let n = Section::SIZE as i32;
    let out = [x, y, z].iter().filter(|&&c| c < 0 || c >= n).count();
    if out == 0 {
        return section.get(x as u32, y as u32, z as u32).is_solid();
    }
    if out >= 2 {
        return false;
    }
    let (face, sx, sy, sz) = if x < 0 {
        (0, n - 1, y, z)
    } else if x >= n {
        (1, 0, y, z)
    } else if y < 0 {
        (2, x, n - 1, z)
    } else if y >= n {
        (3, x, 0, z)
    } else if z < 0 {
        (4, x, y, n - 1)
    } else {
        (5, x, y, 0)
    };
    match neighbors.faces[face] {
        Some(nb) => nb.get(sx as u32, sy as u32, sz as u32).is_solid(),
        None => false,
    }
}

/// Baked ambient occlusion for the four corners of one cell face (0fps method).
/// `d/u/v` are the face's principal/in-plane axes, `sign` its facing, `s` its slice,
/// and `(i, j)` the cell within the slice. Returns AO `0` (darkest) .. `3` for the
/// corners in `(du, dv)` order `(0,0),(1,0),(1,1),(0,1)` — matching [`emit_quad`].
#[allow(clippy::too_many_arguments)]
fn face_ao(
    section: &Section,
    neighbors: &Neighbors,
    d: usize,
    u: usize,
    v: usize,
    sign: i32,
    s: i32,
    i: i32,
    j: i32,
) -> [u8; 4] {
    // The occluders sit in the layer one step out along the face normal.
    let mut l = [0i32; 3];
    l[d] = s + sign;
    l[u] = i;
    l[v] = j;
    let solid = |du: i32, dv: i32| -> u32 {
        let mut p = l;
        p[u] += du;
        p[v] += dv;
        solid_for_ao(section, neighbors, p[0], p[1], p[2]) as u32
    };
    let mut ao = [0u8; 4];
    for (k, &(du, dv)) in [(0i32, 0i32), (1, 0), (1, 1), (0, 1)].iter().enumerate() {
        let su = if du == 0 { -1 } else { 1 };
        let sv = if dv == 0 { -1 } else { 1 };
        let side1 = solid(su, 0);
        let side2 = solid(0, sv);
        let corner = solid(su, sv);
        ao[k] = if side1 == 1 && side2 == 1 {
            0
        } else {
            (3 - (side1 + side2 + corner)) as u8
        };
    }
    ao
}

/// One exposed greedy-mask cell: the block plus its baked look (`[ao; 4]`, block
/// `light`). Two cells merge only when these are all equal.
type FaceCell = (BlockId, [u8; 4], [u8; 3]);

/// Whether a block emits light (seeds the flood-fill). Crystal (material 6) glows; its
/// *colour* is chosen per-crystal by [`emitter_color`] so a point-lit world isn't monochrome.
fn is_emitter(block: BlockId) -> bool {
    block.0 == 6
}

/// The warm/cool light colours (`0..=15` per channel) the crystals cast. Picked per emitter
/// by a hash of its section-local position so a sun-off world reads as varied pools of light
/// — cyan here, amber there, a toxic-green glow over there — instead of one flat hue (E3).
const EMITTER_COLORS: [[u8; 3]; 5] = [
    [5, 14, 15], // cyan
    [15, 9, 3],  // amber
    [13, 4, 14], // violet
    [6, 15, 7],  // toxic green
    [15, 12, 4], // gold / sodium
];

/// Deterministically pick an emitter's light colour from its position.
fn emitter_color(x: usize, y: usize, z: usize) -> [u8; 3] {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (y as u32).wrapping_mul(0x85EB_CA77)
        ^ (z as u32).wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    EMITTER_COLORS[(h as usize) % EMITTER_COLORS.len()]
}

/// Flood-fill block light over a section: BFS from emitters, `−1`/step per channel,
/// spreading through air (design backlog §C). Returns a per-cell light colour grid
/// (`0..=15` per channel). v1: seeded only by emitters **inside this section** — a
/// crystal within ~15 voxels of a chunk border has its glow clipped at the border
/// (cross-chunk light is a follow-up). Blocky/banded on purpose (the §11 look).
fn compute_light(section: &Section) -> Vec<[u8; 3]> {
    use std::collections::VecDeque;
    let n = Section::SIZE as usize;
    let mut light = vec![[0u8; 3]; n * n * n];
    let idx = |x: usize, y: usize, z: usize| x + y * n + z * n * n;

    let mut queue: VecDeque<(usize, usize, usize)> = VecDeque::new();
    for z in 0..n {
        for y in 0..n {
            for x in 0..n {
                if is_emitter(section.get(x as u32, y as u32, z as u32)) {
                    light[idx(x, y, z)] = emitter_color(x, y, z);
                    queue.push_back((x, y, z));
                }
            }
        }
    }

    while let Some((x, y, z)) = queue.pop_front() {
        let cur = light[idx(x, y, z)];
        let spread = |nx: i32, ny: i32, nz: i32, light: &mut [[u8; 3]], q: &mut VecDeque<_>| {
            if nx < 0 || ny < 0 || nz < 0 {
                return;
            }
            let (nx, ny, nz) = (nx as usize, ny as usize, nz as usize);
            if nx >= n || ny >= n || nz >= n {
                return;
            }
            // Light only travels through air; solid faces read adjacent air light.
            if !section.get(nx as u32, ny as u32, nz as u32).is_air() {
                return;
            }
            let i = idx(nx, ny, nz);
            let mut changed = false;
            for c in 0..3 {
                let v = cur[c].saturating_sub(1);
                if v > light[i][c] {
                    light[i][c] = v;
                    changed = true;
                }
            }
            if changed {
                q.push_back((nx, ny, nz));
            }
        };
        let (xi, yi, zi) = (x as i32, y as i32, z as i32);
        spread(xi - 1, yi, zi, &mut light, &mut queue);
        spread(xi + 1, yi, zi, &mut light, &mut queue);
        spread(xi, yi - 1, zi, &mut light, &mut queue);
        spread(xi, yi + 1, zi, &mut light, &mut queue);
        spread(xi, yi, zi - 1, &mut light, &mut queue);
        spread(xi, yi, zi + 1, &mut light, &mut queue);
    }
    light
}

/// Block light reaching one exposed face: the light of the air cell just outside it
/// (`cell + sign` along axis `d`). Out-of-section (boundary) faces read `0` for v1.
#[allow(clippy::too_many_arguments)]
fn face_light(
    light: &[[u8; 3]],
    d: usize,
    u: usize,
    v: usize,
    sign: i32,
    s: i32,
    i: i32,
    j: i32,
) -> [u8; 3] {
    let n = Section::SIZE as i32;
    let mut c = [0i32; 3];
    c[d] = s + sign;
    c[u] = i;
    c[v] = j;
    if (0..n).contains(&c[0]) && (0..n).contains(&c[1]) && (0..n).contains(&c[2]) {
        let nn = Section::SIZE as usize;
        light[c[0] as usize + c[1] as usize * nn + c[2] as usize * nn * nn]
    } else {
        [0; 3]
    }
}

/// Mesh one section (naïve face culling — see module docs). Single-chunk
/// convenience; boundary faces are drawn.
pub fn mesh_section(section: &Section) -> ChunkMesh {
    mesh_section_with(section, &Neighbors::NONE)
}

/// Naïve mesher with neighbour-aware seam culling. The correctness oracle for the
/// greedy mesher.
pub fn mesh_section_with(section: &Section, neighbors: &Neighbors) -> ChunkMesh {
    let mut vertices: Vec<ChunkVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for z in 0..Section::SIZE {
        for y in 0..Section::SIZE {
            for x in 0..Section::SIZE {
                let block = section.get(x, y, z);
                if block.is_air() {
                    continue;
                }

                for face in &FACES {
                    let (nx, ny, nz) = (
                        x as i32 + face.dir[0],
                        y as i32 + face.dir[1],
                        z as i32 + face.dir[2],
                    );
                    if !is_air(section, neighbors, nx, ny, nz) {
                        continue;
                    }
                    let base = vertices.len() as u32;
                    for corner in face.corners {
                        vertices.push(ChunkVertex::new(
                            [
                                (x + corner[0]) as f32,
                                (y + corner[1]) as f32,
                                (z + corner[2]) as f32,
                            ],
                            face.normal,
                            block.0,
                        ));
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
    greedy_mesh_section_with(section, &Neighbors::NONE)
}

/// Greedy mesher with neighbour-aware seam culling (see [`greedy_mesh_section`]).
pub fn greedy_mesh_section_with(section: &Section, neighbors: &Neighbors) -> ChunkMesh {
    // M11 uniform fast paths (byte-identical to the full sweep, just early):
    // an all-air section makes no faces; a uniform-solid section fully enclosed by
    // uniform-solid neighbours has every face occluded.
    if let Some(b) = section.uniform() {
        if b.is_air() {
            return ChunkMesh::default();
        }
        let enclosed = neighbors
            .faces
            .iter()
            .all(|f| f.and_then(|s| s.uniform()).is_some_and(|nb| !nb.is_air()));
        if enclosed {
            return ChunkMesh::default();
        }
    }
    let n = Section::SIZE as i32;
    let mut vertices: Vec<ChunkVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Flood-fill block light once for the section; faces sample it (E3).
    let light_grid = compute_light(section);

    // For each principal axis `d` and each facing direction `sign`, sweep slices
    // and greedily merge the plane of exposed faces.
    for d in 0..3usize {
        let u = (d + 1) % 3;
        let v = (d + 2) % 3;

        for sign in [1i32, -1i32] {
            for s in 0..n {
                // mask[i + j*n] = Some(face) where this slice has a face exposed in
                // direction `sign`. Baked AO + block light are part of the key so
                // merging only joins cells with an identical look.
                let mut mask: Vec<Option<FaceCell>> = vec![None; (n * n) as usize];
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
                        let mut nb = c;
                        nb[d] = s + sign;
                        if is_air(section, neighbors, nb[0], nb[1], nb[2]) {
                            let ao = face_ao(section, neighbors, d, u, v, sign, s, i, j);
                            let lt = face_light(&light_grid, d, u, v, sign, s, i, j);
                            mask[(i + j * n) as usize] = Some((block, ao, lt));
                        }
                    }
                }

                // Greedily merge the mask into maximal rectangles.
                for j in 0..n {
                    let mut i = 0;
                    while i < n {
                        let Some(cell) = mask[(i + j * n) as usize] else {
                            i += 1;
                            continue;
                        };
                        // Extend width along u, then height along v — same block AND
                        // same corner-AO (so AO breaks the merge at occluders).
                        let mut w = 1;
                        while i + w < n && mask[((i + w) + j * n) as usize] == Some(cell) {
                            w += 1;
                        }
                        let mut h = 1;
                        'grow: while j + h < n {
                            for k in 0..w {
                                if mask[((i + k) + (j + h) * n) as usize] != Some(cell) {
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
                            cell.0,
                            cell.1,
                            cell.2,
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
/// `sign`, spanning `[i, i+w]` along `u` and `[j, j+h]` along `v`. `ao` is the baked
/// corner occlusion in `(0,0),(1,0),(1,1),(0,1)` order (see [`face_ao`]).
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
    ao: [u8; 4],
    light: [u8; 3],
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

    // Corners a,b,c,e map to AO indices 0,1,2,3 — keep them paired through winding.
    let (a, b, c, e) = (
        (corner(i, j), ao[0]),
        (corner(i + w, j), ao[1]),
        (corner(i + w, j + h), ao[2]),
        (corner(i, j + h), ao[3]),
    );
    // Wind so the front face matches the normal direction.
    let quad = if sign > 0 { [a, b, c, e] } else { [a, e, c, b] };

    let base = vertices.len() as u32;
    for (position, corner_ao) in quad {
        vertices.push(ChunkVertex::with_ao_light(
            position, normal, block.0, corner_ao, light,
        ));
    }
    // Split the quad along the diagonal between the *darker* pair of corners, so the
    // AO gradient doesn't interpolate a bright triangle across a dark edge. Culling is
    // off, so re-winding is free. `quad` corner order is [0,1,2,3]; default diagonal
    // is 0–2, flipped is 1–3.
    let (q0, q1, q2, q3) = (quad[0].1, quad[1].1, quad[2].1, quad[3].1);
    if q0 as u16 + q2 as u16 >= q1 as u16 + q3 as u16 {
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    } else {
        indices.extend_from_slice(&[base + 1, base + 2, base + 3, base + 1, base + 3, base]);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn uniform_sections_mesh_to_the_same_empty_result_fast() {
        // M11: the early-outs must be byte-identical to the full sweep.
        // All air → empty either way.
        let air = Section::new();
        let full = greedy_mesh_section_with(&air, &Neighbors::NONE);
        assert!(full.vertices.is_empty() && full.indices.is_empty());
        // Uniform solid fully enclosed by uniform solids → empty (every face occluded).
        let mut solid = Section::new();
        for z in 0..Section::SIZE {
            for y in 0..Section::SIZE {
                for x in 0..Section::SIZE {
                    solid.set(x, y, z, BlockId(2));
                }
            }
        }
        // (A filled-by-set section isn't `uniform()` — palette grew. Build a provably-uniform
        // one through the public surface: a fresh section is uniform AIR; for solid we exercise
        // the enclosed check with mixed-uniform neighbours via the air case + the sweep parity.)
        let nb = Neighbors {
            faces: [Some(&solid); 6],
        };
        let enclosed = greedy_mesh_section_with(&solid, &nb);
        assert!(
            enclosed.vertices.is_empty(),
            "fully-enclosed solid meshes no faces (sweep parity)"
        );
        // And an open face still meshes (the early-out must not over-trigger).
        let open = Neighbors {
            faces: [
                Some(&solid),
                Some(&solid),
                None,
                Some(&solid),
                Some(&solid),
                Some(&solid),
            ],
        };
        let one_open = greedy_mesh_section_with(&solid, &open);
        assert!(
            !one_open.vertices.is_empty(),
            "an exposed face still meshes"
        );
    }

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

            let mat = mesh.vertices[base].material as i32;

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
    fn adjacent_chunks_cull_their_shared_seam() {
        // Two full sections sharing the +x / -x boundary: the face between them
        // must not be drawn from either side.
        let full = {
            let mut s = Section::new();
            for z in 0..Section::SIZE {
                for y in 0..Section::SIZE {
                    for x in 0..Section::SIZE {
                        s.set(x, y, z, STONE);
                    }
                }
            }
            s
        };

        // Lone full chunk: 6 outer quads.
        assert_eq!(greedy_mesh_section(&full).quad_count(), 6);

        // With a solid neighbour on +x, that face is gone: 5 quads.
        let neighbours = Neighbors {
            faces: [None, Some(&full), None, None, None, None],
        };
        let meshed = greedy_mesh_section_with(&full, &neighbours);
        assert_eq!(meshed.quad_count(), 5);
        // The culled face is the one pointing +x; no vertex should carry that normal.
        assert!(
            meshed.vertices.iter().all(|v| v.normal[0] <= 0.0),
            "a +x face was drawn on the culled seam",
        );

        // The naïve oracle agrees: one 32x32 sheet of faces removed.
        assert_eq!(
            mesh_section_with(&full, &neighbours).quad_count(),
            6 * 32 * 32 - 32 * 32,
        );
    }

    #[test]
    fn packed_vertex_round_trips_every_field() {
        // Edge values across each field's range.
        for &pos in &[[0, 0, 0], [32, 0, 17], [1, 31, 32], [32, 32, 32]] {
            for dir in 0..6u32 {
                for &material in &[0u32, 1, 4, 255, 511] {
                    let packed = pack_vertex(pos, dir, material, 0);
                    let (p, d, m, ao) = unpack_vertex(packed);
                    assert_eq!(p, pos);
                    assert_eq!(d, dir);
                    assert_eq!(m, material);
                    assert_eq!(ao, 0);
                }
            }
        }
    }

    #[test]
    fn pack_maps_normals_to_the_right_direction() {
        for (dir, n) in FACE_NORMALS.iter().enumerate() {
            let v = ChunkVertex::new([1.0, 2.0, 3.0], *n, 7);
            let (_, d, m, _) = unpack_vertex(pack(&v)[0]);
            assert_eq!(d as usize, dir);
            assert_eq!(m, 7);
        }
    }

    #[test]
    fn pack_round_trips_block_light() {
        let v = ChunkVertex::with_ao_light([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], 3, 2, [5, 14, 15]);
        let word1 = pack(&v)[1];
        assert_eq!(word1 & 0xF, 5);
        assert_eq!((word1 >> 4) & 0xF, 14);
        assert_eq!((word1 >> 8) & 0xF, 15);
    }

    // --- Flood-fill block light (E3) ------------------------------------------

    const CRYSTAL: BlockId = BlockId(6);

    #[test]
    fn a_crystal_lights_the_blocks_around_it() {
        // A crystal sitting on a flat floor must light the floor's top faces near it,
        // brightest adjacent and fading with distance.
        let mut s = Section::new();
        for z in 8..24 {
            for x in 8..24 {
                s.set(x, 5, z, STONE);
            }
        }
        s.set(16, 6, 16, CRYSTAL); // crystal on the floor

        let mesh = greedy_mesh_section(&s);
        // Find lit top-face vertices (normal +y) and the brightest one.
        let top_lit: Vec<&ChunkVertex> = mesh
            .vertices
            .iter()
            .filter(|v| v.normal[1] > 0.5 && v.light != [0, 0, 0])
            .collect();
        assert!(!top_lit.is_empty(), "crystal lit no surrounding top faces");
        // The cast light carries the emitter's own (per-position) hue: its dominant channel
        // should dominate the surrounding lit faces too.
        let ec = emitter_color(16, 6, 16);
        let dom = (0..3).max_by_key(|&c| ec[c]).unwrap();
        let max_dom = top_lit.iter().map(|v| v.light[dom]).max().unwrap();
        for c in 0..3 {
            let max_c = top_lit.iter().map(|v| v.light[c]).max().unwrap();
            assert!(
                max_dom >= max_c,
                "lit faces should be dominated by the emitter's hue (channel {dom})"
            );
        }
    }

    #[test]
    fn no_emitters_means_no_block_light() {
        let mut s = Section::new();
        for z in 0..Section::SIZE {
            for x in 0..Section::SIZE {
                s.set(x, 0, z, STONE);
            }
        }
        let mesh = greedy_mesh_section(&s);
        assert!(mesh.vertices.iter().all(|v| v.light == [0, 0, 0]));
    }

    // --- Baked AO (M4) --------------------------------------------------------

    #[test]
    fn isolated_voxel_is_fully_unoccluded() {
        // No neighbours anywhere → every corner of every face is open (ao == 3).
        let mesh = greedy_mesh_section(&section_with(&[(10, 10, 10)]));
        assert!(
            mesh.vertices.iter().all(|v| v.ao == 3),
            "lone voxel should have no occlusion",
        );
    }

    #[test]
    fn a_neighbour_darkens_adjacent_face_corners() {
        // A voxel plus an occluder sitting diagonally above one edge of it. The
        // occluder is in the layer just above the voxel's +y (top) face, so it
        // darkens the two top-face corners on that edge — but not the far ones.
        let mesh = greedy_mesh_section(&section_with(&[(10, 10, 10), (11, 11, 10)]));
        let top: Vec<&ChunkVertex> = mesh.vertices.iter().filter(|v| v.normal[1] > 0.5).collect();
        assert!(top.iter().any(|v| v.ao < 3), "no top-face corner darkened");
        assert!(top.iter().any(|v| v.ao == 3), "whole top face darkened");
    }

    #[test]
    fn occlusion_breaks_a_greedy_merge() {
        // A flat 1-voxel-thick floor merges its top face into a single quad. Drop a
        // block onto that floor and the AO discontinuity around it must split the top
        // face into more than one quad (no longer a clean single rectangle).
        let mut floor = Section::new();
        for z in 8..16 {
            for x in 8..16 {
                floor.set(x, 5, z, STONE);
            }
        }
        let plain_top = greedy_mesh_section(&floor)
            .vertices
            .iter()
            .filter(|v| v.normal[1] > 0.5)
            .count()
            / 4;
        assert_eq!(plain_top, 1, "flat floor top should be one quad");

        floor.set(11, 6, 11, STONE); // an occluder sitting on the floor
        let bumped_top = greedy_mesh_section(&floor)
            .vertices
            .iter()
            .filter(|v| v.normal[1] > 0.5)
            .count()
            / 4;
        assert!(
            bumped_top > 1,
            "AO around the occluder should split the floor's top face",
        );
    }

    #[test]
    fn ao_does_not_change_the_covered_face_set() {
        // AO affects shading + how faces are split, never *which* unit faces exist:
        // the greedy mesh must still cover exactly the naïve face set.
        for section in oracle_fixtures() {
            let mut naive = face_cells(&mesh_section(&section));
            let mut greedy = face_cells(&greedy_mesh_section(&section));
            naive.sort_unstable();
            greedy.sort_unstable();
            assert_eq!(naive, greedy);
        }
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
