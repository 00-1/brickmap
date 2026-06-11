//! M10 — **performance budgets & reference-scene gates** (the [performance charter]'s §5).
//! The frame's *content* cost — chunks, triangles, splats, labels — is computed CPU-side for
//! fixed reference scenes (deterministic per seed: same streamed set, same meshes), and CI
//! asserts each counter stays under its pinned budget, so game-content accretion that would
//! eat the weak-hardware headroom **fails the build here** — months before a human feels it.
//!
//! Budgets are pinned at **measured actual + headroom** (loose first pins per the charter —
//! they exist to catch *step changes*, not to be a straitjacket; tightened at M8b). A failing
//! gate must never be "fixed" by deleting it — adjust the budget with a recorded reason.
//!
//! Live-only counters (upload bytes/frame, mesh-queue depth, inline-mesh ms) are surfaced on
//! the HUD via the engine's `DrawStats` but are **not** CI-gated (timing-dependent — the
//! brief's Decision 4); the content counters here are the deterministic regression anchor.
//!
//! [performance charter]: ../docs/performance.md

use glam::Vec3;

use crate::{biome, structures, world, worldgen};

// ---- the budget table — pinned at MEASURED ACTUAL + ~40% headroom (brief Decision 2) -------
//
// Measured 2026-06-11 (seed 1337, `cargo run --bin stats`; full streamed-set totals,
// camera-independent — culling only reduces them):
//   scene    tris_total  splats_total  mesh_draws  labels
//   default   1,794,750      123,358        169       7
//   forest    1,888,548       78,309        832       8   ← solid giants = 663 mesh sections!
//   giant     1,739,366      104,294        169       9
// The charter §6 *estimates* (1.5 M tris / ≤400 draws) were placeholders; these pins replace
// them (recorded in performance.md §6). Worst-case scene actual + ~40%:

/// Triangles: streamed terrain + solid structures (worst actual 1.89 M @ forest).
pub const TRIANGLE_BUDGET: u32 = 2_600_000;
/// Live splats: foliage + structure points + wisps (worst actual 123 k @ default).
pub const SPLAT_BUDGET: u32 = 170_000;
/// Mesh draw instances: streamed chunks + solid-structure sections (worst actual 832 @ forest —
/// solid giants dominate; frustum culling roughly halves this live. A real M8b lever.)
pub const MESH_DRAW_BUDGET: u32 = 1_200;
/// In-range inscription labels (each a small texture + a quad batch; worst actual 9).
pub const LABEL_BUDGET: u32 = 16;

// ---- per-consumer splat budgets (charter §4 rule 2: declared at the call site) --------------

/// Ground foliage + trees/bushes, across the whole streamed set (worst actual 121.5 k).
pub const FOLIAGE_SPLAT_BUDGET: u32 = 170_000;
/// Ethereal structure points (tube-tech relics + fallen humans) in range (worst actual 2.8 k).
pub const STRUCTURE_SPLAT_BUDGET: u32 = 4_000;
/// Drifting wisps (E15): a hard cap on the per-frame swarm upload (a cap, not a measurement).
pub const WISP_SPLAT_BUDGET: u32 = 64;

/// The deterministic *content* counters for one reference scene — what the renderer would be
/// handed with the streamed set fully loaded around `pos` (camera-independent totals: culling
/// only reduces them, and content regressions move these, not the culling).
#[derive(Debug, Default, Clone, Copy)]
pub struct SceneStats {
    pub chunks: u32,
    pub triangles: u32,
    pub foliage_splats: u32,
    pub structure_points: u32,
    pub structure_tris: u32,
    pub structure_meshes: u32,
    pub labels: u32,
    pub label_glyphs: u32,
}

impl SceneStats {
    /// Total live splats this scene asks of the splat pipeline (+ the wisp cap).
    pub fn splats(&self) -> u32 {
        self.foliage_splats + self.structure_points + WISP_SPLAT_BUDGET
    }
    /// Total triangles (terrain + solid structures).
    pub fn tris(&self) -> u32 {
        self.triangles + self.structure_tris
    }
    /// Mesh draw calls (one per chunk + one per solid-structure section).
    pub fn mesh_draws(&self) -> u32 {
        self.chunks + self.structure_meshes
    }
    /// One `key=value` per line — the machine-readable form the `stats` bin prints.
    pub fn report(&self, scene: &str) -> String {
        format!(
            "scene={scene}\nchunks={}\ntriangles={}\nfoliage_splats={}\nstructure_points={}\nstructure_tris={}\nstructure_meshes={}\nlabels={}\nlabel_glyphs={}\nsplats_total={}\ntris_total={}\nmesh_draws={}\n",
            self.chunks,
            self.triangles,
            self.foliage_splats,
            self.structure_points,
            self.structure_tris,
            self.structure_meshes,
            self.labels,
            self.label_glyphs,
            self.splats(),
            self.tris(),
            self.mesh_draws(),
        )
    }
}

/// Compute the content counters for the streamed world around `pos` (the live `stream()` keep
/// set: a `(2·(STREAM_RADIUS+1)+1)²` chunk square, one y layer) + the in-range structures and
/// inscriptions. Deterministic in `(seed, pos)`.
pub fn scene_stats(seed: u32, pos: Vec3) -> SceneStats {
    let mut st = SceneStats::default();
    let s = world::Section::SIZE as f32;
    let (ccx, ccz) = ((pos.x / s).floor() as i32, (pos.z / s).floor() as i32);
    let keep = crate::STREAM_RADIUS + 1;
    let mut cache = std::collections::HashMap::new(); // each section generated once, not 5×
    for dz in -keep..=keep {
        for dx in -keep..=keep {
            let inst =
                crate::build_chunk_instance_cached((ccx + dx, 0, ccz + dz), seed, &mut cache);
            if inst.mesh.indices.is_empty() {
                continue;
            }
            st.chunks += 1;
            st.triangles += inst.mesh.indices.len() as u32 / 3;
            st.foliage_splats += inst.foliage.len() as u32;
        }
    }
    let ground = |x: f32, z: f32| worldgen::height(x.floor() as i32, z.floor() as i32, seed) as f32;
    for p in structures::colossi_near(seed, pos, crate::STRUCTURE_RADIUS, ground) {
        let (points, meshes) = crate::structure_geometry(&p);
        st.structure_points += points.len() as u32;
        for inst in &meshes {
            st.structure_tris += inst.mesh.indices.len() as u32 / 3;
        }
        st.structure_meshes += meshes.len() as u32;
        st.labels += 1; // the monument label
        st.label_glyphs += structures::colossus_label(&p).text.chars().count() as u32;
    }
    for m in structures::inscriptions_near(seed, pos, crate::TEXT_RADIUS, ground) {
        st.labels += 1;
        st.label_glyphs += m.text.chars().count() as u32;
    }
    st
}

/// The three reference scenes (brief Decision 3), chosen from the world the headless tooling
/// already frames; positions are deterministic per seed and documented here:
/// 1. `default` — the spawn vantage (origin at cruise height) on the default seed.
/// 2. `forest` — the densest-forest cell within ±2 km of origin (deterministic argmax).
/// 3. `giant` — at the nearest colossus to origin (structure-heavy close-up).
pub fn reference_scenes(seed: u32) -> [(&'static str, Vec3); 3] {
    let ground = |x: f32, z: f32| worldgen::height(x.floor() as i32, z.floor() as i32, seed) as f32;
    let cruise = |x: f32, z: f32| Vec3::new(x, ground(x, z) + 22.0, z);
    // Deterministic densest-forest scan (coarse grid; ties broken by scan order).
    let mut best = (0.0f32, 0.0f32, f32::MIN);
    let mut xz = -2000.0f32;
    while xz <= 2000.0 {
        let mut z = -2000.0f32;
        while z <= 2000.0 {
            let f = biome::density(xz, z, seed).1;
            if f > best.2 {
                best = (xz, z, f);
            }
            z += 250.0;
        }
        xz += 250.0;
    }
    // Nearest colossus to origin (fall back to origin if none in 1.5 km — never on real seeds).
    let giant = structures::colossi_near(seed, Vec3::ZERO, 1500.0, ground)
        .into_iter()
        .min_by(|a, b| a.pos.length_squared().total_cmp(&b.pos.length_squared()))
        .map(|p| p.pos)
        .unwrap_or(Vec3::ZERO);
    [
        ("default", cruise(0.0, 0.0)),
        ("forest", cruise(best.0, best.1)),
        ("giant", cruise(giant.x, giant.z)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The M10 gate: every reference scene's content counters stay under budget. A failure names
    /// the counter, the scene, and by how much — if content legitimately grows, adjust the budget
    /// const above WITH a recorded reason (never delete the gate).
    #[test]
    fn reference_scenes_stay_under_budget() {
        for (name, pos) in reference_scenes(crate::WORLD_SEED) {
            let st = scene_stats(crate::WORLD_SEED, pos);
            let checks: [(&str, u32, u32); 5] = [
                ("triangles", st.tris(), TRIANGLE_BUDGET),
                ("splats", st.splats(), SPLAT_BUDGET),
                ("mesh_draws", st.mesh_draws(), MESH_DRAW_BUDGET),
                ("labels", st.labels, LABEL_BUDGET),
                ("foliage_splats", st.foliage_splats, FOLIAGE_SPLAT_BUDGET),
            ];
            for (counter, actual, budget) in checks {
                assert!(
                    actual <= budget,
                    "BUDGET BLOWN [{name}] {counter}: {actual} > {budget} (over by {})— \
                     see docs/performance.md; adjust the budget only with a recorded reason",
                    actual - budget.min(actual)
                );
            }
            // Structure points respect their consumer budget too.
            assert!(
                st.structure_points <= STRUCTURE_SPLAT_BUDGET,
                "BUDGET BLOWN [{name}] structure_points: {} > {STRUCTURE_SPLAT_BUDGET}",
                st.structure_points
            );
        }
    }

    /// The counters are deterministic (same seed + pos → same numbers) — the property that makes
    /// them CI-assertable at all.
    #[test]
    fn scene_stats_are_deterministic() {
        let pos = Vec3::new(0.0, 40.0, 0.0);
        let a = scene_stats(crate::WORLD_SEED, pos);
        let b = scene_stats(crate::WORLD_SEED, pos);
        assert_eq!(a.tris(), b.tris());
        assert_eq!(a.splats(), b.splats());
        assert_eq!(a.labels, b.labels);
        // And the scene set itself is stable.
        assert_eq!(reference_scenes(1337)[1].1, reference_scenes(1337)[1].1);
    }
}
