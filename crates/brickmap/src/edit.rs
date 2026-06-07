//! Voxel editing (E14) routed through a small **command/event seam** — the low-regret
//! prep the multiplayer research flagged (a single serialisable mutation type + one
//! `apply`, so the same events later drive undo/replay, seed+delta sharing, and network
//! broadcast). Pure logic: a DDA voxel raycast for picking, and `apply` over the live
//! `overlay` (the same sparse `ChunkCoord -> Section` map the falling sand uses).
//!
//! `Edit` is deliberately plain-old-data (coords + a block id), so it's trivially
//! serialisable later (hand-rolled like `share`, or via serde when we add it) — we keep
//! the *shape* right now without taking the dependency yet.

use std::collections::HashMap;

use glam::Vec3;

use crate::world::{BlockId, ChunkCoord, Section};

/// One world-mutating edit. The whole editing/sharing/broadcast story funnels through
/// this enum and [`apply`] — nothing else pokes the overlay directly.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Edit {
    /// Set the voxel at a world position to `block` (use `BlockId::AIR` to remove).
    Set { pos: [i32; 3], block: BlockId },
}

/// Split a world voxel position into its chunk coord + in-section local coords. Returns
/// `None` if `y` is outside the single vertical chunk layer (`0..SIZE`) this world uses.
pub fn world_to_chunk_local(pos: [i32; 3]) -> Option<(ChunkCoord, u32, u32, u32)> {
    let s = Section::SIZE as i32;
    if pos[1] < 0 || pos[1] >= s {
        return None;
    }
    let coord = (pos[0].div_euclid(s), 0, pos[2].div_euclid(s));
    let lx = pos[0].rem_euclid(s) as u32;
    let lz = pos[2].rem_euclid(s) as u32;
    Some((coord, lx, pos[1] as u32, lz))
}

/// Apply an edit to the `overlay`, materialising the affected section from the seed if it
/// isn't already overlaid. Returns the dirtied `ChunkCoord` and the **previous** block
/// (so callers can record an inverse edit for undo), or `None` if out of range / no-op.
pub fn apply(
    overlay: &mut HashMap<ChunkCoord, Section>,
    seed: u32,
    edit: &Edit,
) -> Option<(ChunkCoord, Edit)> {
    let Edit::Set { pos, block } = *edit;
    let (coord, lx, ly, lz) = world_to_chunk_local(pos)?;
    let sec = overlay
        .entry(coord)
        .or_insert_with(|| crate::worldgen::generate_section(coord.0, coord.2, seed));
    let prev = sec.get(lx, ly, lz);
    if prev == block {
        return None; // no-op: don't dirty or log
    }
    sec.set(lx, ly, lz, block);
    Some((coord, Edit::Set { pos, block: prev }))
}

/// A DDA (Amanatides–Woo) ray–voxel hit: the solid voxel struck and the face normal of
/// the cell we entered through (the empty neighbour, for placing *against* a surface).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub voxel: [i32; 3],
    pub normal: [i32; 3],
}

/// March a ray through the voxel grid, returning the first cell for which `solid` is true
/// (within `max_dist` world units). `solid` answers "is this voxel filled?" — the caller
/// wires it to base terrain + overlay. Branch-light grid traversal, no allocation.
pub fn raycast(
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
    solid: impl Fn([i32; 3]) -> bool,
) -> Option<Hit> {
    let dir = dir.normalize_or_zero();
    if dir == Vec3::ZERO {
        return None;
    }
    let mut voxel = [
        origin.x.floor() as i32,
        origin.y.floor() as i32,
        origin.z.floor() as i32,
    ];
    let step = [
        if dir.x >= 0.0 { 1 } else { -1 },
        if dir.y >= 0.0 { 1 } else { -1 },
        if dir.z >= 0.0 { 1 } else { -1 },
    ];
    let inv = [
        1.0 / dir.x.abs().max(1e-9),
        1.0 / dir.y.abs().max(1e-9),
        1.0 / dir.z.abs().max(1e-9),
    ];
    let o = [origin.x, origin.y, origin.z];
    let d = [dir.x, dir.y, dir.z];
    // Distance along the ray to the next grid line on each axis.
    let mut t_max = [0.0f32; 3];
    for a in 0..3 {
        let next = if d[a] >= 0.0 {
            (voxel[a] as f32 + 1.0) - o[a]
        } else {
            o[a] - voxel[a] as f32
        };
        t_max[a] = next * inv[a];
    }
    let t_delta = inv;

    // If we start inside a solid voxel, that's the hit (normal faces back along the ray).
    if solid(voxel) {
        return Some(Hit {
            voxel,
            normal: [-step[0], 0, 0], // arbitrary-ish; caller rarely places from inside
        });
    }
    let mut t = 0.0f32;
    while t <= max_dist {
        // Advance along the axis whose next grid line is nearest.
        let last_axis = if t_max[0] < t_max[1] {
            if t_max[0] < t_max[2] {
                0
            } else {
                2
            }
        } else if t_max[1] < t_max[2] {
            1
        } else {
            2
        };
        voxel[last_axis] += step[last_axis];
        t = t_max[last_axis];
        t_max[last_axis] += t_delta[last_axis];
        if solid(voxel) {
            let mut normal = [0, 0, 0];
            normal[last_axis] = -step[last_axis]; // face we entered through
            return Some(Hit { voxel, normal });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const STONE: BlockId = BlockId(1);

    #[test]
    fn world_to_chunk_local_wraps_negatives() {
        // x = -1 → chunk -1, local SIZE-1.
        let (c, lx, ly, lz) = world_to_chunk_local([-1, 5, 33]).unwrap();
        assert_eq!(c, (-1, 0, 1));
        assert_eq!((lx, ly, lz), (Section::SIZE - 1, 5, 1));
        // Out of the single vertical layer → None.
        assert!(world_to_chunk_local([0, -1, 0]).is_none());
        assert!(world_to_chunk_local([0, Section::SIZE as i32, 0]).is_none());
    }

    #[test]
    fn apply_sets_voxel_and_returns_inverse() {
        let mut overlay = HashMap::new();
        let set = Edit::Set {
            pos: [3, 10, 3],
            block: STONE,
        };
        let (coord, inverse) = apply(&mut overlay, 1337, &set).unwrap();
        assert_eq!(coord, (0, 0, 0));
        // The voxel is now stone.
        assert_eq!(overlay[&coord].get(3, 10, 3), STONE);
        // Applying the inverse restores the original block (and dirties the same chunk).
        let (coord2, _) = apply(&mut overlay, 1337, &inverse).unwrap();
        assert_eq!(coord2, coord);
        assert_ne!(overlay[&coord].get(3, 10, 3), STONE);
    }

    #[test]
    fn apply_noop_when_block_unchanged() {
        let mut overlay = HashMap::new();
        // Air→air on an air cell is a no-op (returns None, no dirty/log).
        let set_air = Edit::Set {
            pos: [1, 20, 1],
            block: BlockId::AIR,
        };
        // A high cell in the demo seed is air; setting it to air should be a no-op.
        assert!(apply(&mut overlay, 1337, &set_air).is_none());
    }

    #[test]
    fn raycast_hits_a_solid_voxel_along_x() {
        // Solid wall at x == 5. Ray from x=0 along +x hits voxel x=5, normal facing -x.
        let hit = raycast(
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(1.0, 0.0, 0.0),
            32.0,
            |v| v[0] == 5,
        )
        .unwrap();
        assert_eq!(hit.voxel, [5, 0, 0]);
        assert_eq!(hit.normal, [-1, 0, 0]);
    }

    #[test]
    fn raycast_misses_empty_space() {
        assert!(raycast(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), 16.0, |_| false).is_none());
    }

    #[test]
    fn raycast_normal_lets_you_place_against_a_floor() {
        // Floor at y == 0 (solid for y <= 0). Ray from above going down hits y=0 with an
        // upward normal, so hit.voxel + normal is the empty cell to place into.
        let hit = raycast(
            Vec3::new(0.5, 8.0, 0.5),
            Vec3::new(0.0, -1.0, 0.0),
            32.0,
            |v| v[1] <= 0,
        )
        .unwrap();
        assert_eq!(hit.normal, [0, 1, 0]);
        let place = [
            hit.voxel[0] + hit.normal[0],
            hit.voxel[1] + hit.normal[1],
            hit.voxel[2] + hit.normal[2],
        ];
        assert_eq!(place, [0, 1, 0]);
    }
}
