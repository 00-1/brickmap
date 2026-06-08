//! Mesh → points for the **human-figure** colossi (E18). A tiny OBJ loader + area-weighted
//! surface sampler turns a CC0 model (the MakeHuman base mesh, `assets/base-human.obj`) into a
//! point cloud, which — laid down, scaled to giant, placed — renders through the splat pipeline
//! exactly like the tube-tech relics. The *human* structure kind, distinct from the procedural
//! relics. Pure logic (no wgpu); the caller supplies the OBJ text and placement.

use glam::Vec3;

use crate::foliage::SplatInstance;

/// A loaded triangle mesh (positions + triangle vertex-index triples).
pub struct Mesh {
    pub verts: Vec<Vec3>,
    pub tris: Vec<[u32; 3]>,
}

/// Parse a Wavefront OBJ from text — just `v` (positions) and `f` (faces, fan-triangulated;
/// polygons/quads handled; only the vertex index of each `v/vt/vn` token is used). Everything
/// else (vt, vn, groups, comments) is ignored. Robust enough for the CC0 base mesh.
pub fn load_obj(text: &str) -> Mesh {
    let mut verts = Vec::new();
    let mut tris = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("v") => {
                let mut f = it.filter_map(|t| t.parse::<f32>().ok());
                if let (Some(x), Some(y), Some(z)) = (f.next(), f.next(), f.next()) {
                    verts.push(Vec3::new(x, y, z));
                }
            }
            Some("f") => {
                let poly: Vec<u32> = it
                    .filter_map(|tok| tok.split('/').next())
                    .filter_map(|s| s.parse::<i64>().ok())
                    .map(|i| (i - 1) as u32) // OBJ is 1-based (assumes positive indices)
                    .collect();
                for k in 1..poly.len().saturating_sub(1) {
                    tris.push([poly[0], poly[k], poly[k + 1]]);
                }
            }
            _ => {}
        }
    }
    Mesh { verts, tris }
}

/// xorshift32 → `[0, 1)`.
struct Rng(u32);
impl Rng {
    fn unit(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// Sample ~`target` points uniformly over the mesh **surface** (area-weighted), deterministic
/// in `seed`. Returns model-space positions.
pub fn surface_points(mesh: &Mesh, target: usize, seed: u32) -> Vec<Vec3> {
    let tri = |t: &[u32; 3]| {
        (
            mesh.verts[t[0] as usize],
            mesh.verts[t[1] as usize],
            mesh.verts[t[2] as usize],
        )
    };
    let area = |t: &[u32; 3]| {
        let (a, b, c) = tri(t);
        0.5 * (b - a).cross(c - a).length()
    };
    let total: f32 = mesh.tris.iter().map(area).sum();
    let density = target as f32 / total.max(1e-6);
    let mut rng = Rng(seed | 1);
    let mut out = Vec::with_capacity(target + 64);
    for t in &mesh.tris {
        let n = area(t) * density;
        let count = n.floor() as usize + usize::from(rng.unit() < n.fract());
        let (a, b, c) = tri(t);
        for _ in 0..count {
            let (mut u, mut v) = (rng.unit(), rng.unit());
            if u + v > 1.0 {
                u = 1.0 - u;
                v = 1.0 - v;
            }
            out.push(a + (b - a) * u + (c - a) * v);
        }
    }
    out
}

/// **Solid voxelisation (E18):** snap a dense surface sampling of `mesh` onto a `res³`-ish grid
/// (the model's longest axis spans ~`res` cells), yielding the **solid surface-shell voxels** as
/// local grid coords `[x,y,z]` (deduplicated, model-space orientation). A shell, not a filled
/// interior — which is what an *explorable* giant wants (you walk its surfaces / into its hollows),
/// and it matches how the tube-tech relics voxelise. Deterministic in `seed`. The caller toples /
/// scales / greedy-meshes it into chunk instances (like the relics' solid path).
pub fn voxelize(mesh: &Mesh, res: u32, seed: u32) -> Vec<[i32; 3]> {
    let res = res.max(1);
    // Model bounds → a uniform scale that maps the longest axis to `res` cells.
    let (mut lo, mut hi) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
    for v in &mesh.verts {
        lo = lo.min(*v);
        hi = hi.max(*v);
    }
    let span = (hi - lo).max_element().max(1e-6);
    let cell = span / res as f32;
    // Oversample so the shell has no pinholes: ~a few samples per surface cell. Surface area
    // scales with span², so target ∝ (res)² with a density factor.
    let target = (res as usize * res as usize * 6).max(256);
    let mut set = std::collections::HashSet::new();
    for p in surface_points(mesh, target, seed) {
        let g = (p - lo) / cell;
        set.insert([g.x.floor() as i32, g.y.floor() as i32, g.z.floor() as i32]);
    }
    let mut out: Vec<[i32; 3]> = set.into_iter().collect();
    out.sort_unstable(); // deterministic order (the sampler is seeded, the set isn't ordered)
    out
}

/// Encode a model-space point cloud to a compact little-endian blob (`u32` count + `3×f32` per
/// point) — the **baked** human asset (E18), so the live build embeds these points instead of
/// shipping + parsing the raw 19k-vert OBJ. Pure; round-trips with [`decode_points`].
pub fn encode_points(pts: &[Vec3]) -> Vec<u8> {
    let mut b = Vec::with_capacity(4 + pts.len() * 12);
    b.extend_from_slice(&(pts.len() as u32).to_le_bytes());
    for p in pts {
        b.extend_from_slice(&p.x.to_le_bytes());
        b.extend_from_slice(&p.y.to_le_bytes());
        b.extend_from_slice(&p.z.to_le_bytes());
    }
    b
}

/// Decode the [`encode_points`] blob back to model-space points (lenient: truncates to whatever
/// whole points are present; empty on a short/garbage blob).
pub fn decode_points(b: &[u8]) -> Vec<Vec3> {
    if b.len() < 4 {
        return Vec::new();
    }
    let n = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize;
    let avail = (b.len() - 4) / 12;
    let n = n.min(avail);
    let f = |o: usize| f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    (0..n)
        .map(|i| {
            let o = 4 + i * 12;
            Vec3::new(f(o), f(o + 4), f(o + 8))
        })
        .collect()
}

/// Lay a Y-up standing model down on the ground and place it as a giant point cloud: topple it
/// onto its back (model +Y → world +Z), scale by `scale`, yaw about +Y, rest its lowest point
/// at `feet`, and tint `color`. Reuses the splat billboard path (ethereal, drift-through).
pub fn fallen_splats(
    model_pts: &[Vec3],
    feet: Vec3,
    scale: f32,
    yaw: f32,
    color: [f32; 3],
    seed: u32,
) -> Vec<SplatInstance> {
    let mut rng = Rng(seed | 1);
    fallen_world(model_pts, feet, scale, yaw)
        .into_iter()
        .map(|world| {
            let v = 0.85 + 0.15 * rng.unit();
            SplatInstance {
                offset: [world.x, world.y, world.z],
                size: scale * 0.10,
                color: [color[0] * v, color[1] * v, color[2] * v],
                sway: rng.unit() * std::f32::consts::TAU,
                alpha: 1.0,
            }
        })
        .collect()
}

/// The shared **fallen-giant transform** (E18): topple a Y-up standing model onto its back
/// (model +Y → world +Z), scale, yaw about +Y, and rest its lowest point at `feet`. Returns the
/// world-space positions. Used by both the ethereal points ([`fallen_splats`]) and the solid
/// voxelisation, so the two kinds of fallen human sit identically.
pub fn fallen_world(model_pts: &[Vec3], feet: Vec3, scale: f32, yaw: f32) -> Vec<Vec3> {
    let (sy, cy) = yaw.sin_cos();
    let toppled = |p: Vec3| Vec3::new(p.x, p.z, -p.y);
    let mut min_y = f32::MAX;
    for &p in model_pts {
        min_y = min_y.min(toppled(p).y * scale);
    }
    model_pts
        .iter()
        .map(|&p| {
            let t = toppled(p) * scale;
            let wx = t.x * cy - t.z * sy;
            let wz = t.x * sy + t.z * cy;
            feet + Vec3::new(wx, t.y - min_y, wz)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // A unit quad (two triangles) in the XY plane, as OBJ (quad face).
    const QUAD: &str = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1/1 2/2 3/3 4/4\n";

    #[test]
    fn parses_verts_and_triangulates_quad() {
        let m = load_obj(QUAD);
        assert_eq!(m.verts.len(), 4);
        assert_eq!(m.tris.len(), 2, "a quad fan-triangulates to 2 tris");
    }

    #[test]
    fn samples_roughly_target_points_on_surface() {
        let m = load_obj(QUAD); // area 1.0
        let p = surface_points(&m, 500, 7);
        assert!(
            (p.len() as i32 - 500).abs() < 80,
            "expected ~500 points, got {}",
            p.len()
        );
        // All points lie in the unit quad (z == 0).
        assert!(p.iter().all(|q| q.z.abs() < 1e-4
            && (-0.01..=1.01).contains(&q.x)
            && (-0.01..=1.01).contains(&q.y)));
    }

    #[test]
    fn voxelize_fills_a_grid_deterministically() {
        // A bigger planar quad voxelises into a flat slab of cells on one grid plane (z = 0),
        // spanning ~res cells in x/y. Deterministic + bounded.
        let big = "v 0 0 0\nv 8 0 0\nv 8 8 0\nv 0 8 0\nf 1 2 3 4\n";
        let m = load_obj(big);
        let a = voxelize(&m, 16, 3);
        let b = voxelize(&m, 16, 3);
        assert_eq!(a, b, "voxelisation is deterministic in seed");
        assert!(!a.is_empty());
        // All cells on the single z-plane, within the [0,res] grid extent.
        assert!(a.iter().all(|c| c[2] == 0));
        let (mut maxx, mut maxy) = (0, 0);
        for c in &a {
            assert!(c[0] >= 0 && c[1] >= 0);
            maxx = maxx.max(c[0]);
            maxy = maxy.max(c[1]);
        }
        assert!(
            maxx >= 8 && maxy >= 8,
            "should span the grid ({maxx},{maxy})"
        );
        assert!(maxx <= 16 && maxy <= 16, "within the resolution");
    }

    #[test]
    fn points_blob_round_trips() {
        let pts = vec![
            Vec3::new(1.0, -2.5, 3.25),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(-9.0, 100.0, 0.001),
        ];
        let back = decode_points(&encode_points(&pts));
        assert_eq!(back, pts);
        assert!(decode_points(&[]).is_empty());
        assert!(decode_points(&[9, 0, 0, 0]).is_empty()); // claims 9 points, none present
    }

    #[test]
    fn fallen_rests_on_the_ground() {
        let m = load_obj(QUAD);
        let pts = surface_points(&m, 200, 1);
        let splats = fallen_splats(&pts, Vec3::new(10.0, 5.0, -3.0), 4.0, 0.7, [0.8; 3], 2);
        let min_y = splats.iter().map(|s| s.offset[1]).fold(f32::MAX, f32::min);
        assert!(
            (min_y - 5.0).abs() < 1e-2,
            "lowest point should sit at feet.y"
        );
    }
}
