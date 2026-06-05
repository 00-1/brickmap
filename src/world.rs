//! Voxel world data model (M1: the minimum that the mesher needs).
//!
//! This is the start of the `world` layer from `docs/architecture.md`: it owns
//! voxel data and **knows nothing about wgpu or how it is drawn**. For M1 storage
//! is a flat dense array; palette compression (design §7.3) replaces the backing
//! store in M3 behind this same `get`/`set` API.

/// A block type. `0` is reserved for empty space (air).
///
/// Real block semantics (names, materials, textures) arrive with the palette in
/// M3; for now a `BlockId` is just an opaque non-zero marker of "something solid".
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct BlockId(pub u16);

impl BlockId {
    /// Empty space. Faces are emitted where a solid block borders air.
    pub const AIR: BlockId = BlockId(0);

    /// Whether this block is empty space.
    #[inline]
    pub fn is_air(self) -> bool {
        self == BlockId::AIR
    }

    /// Whether this block is solid (currently just "not air").
    #[inline]
    pub fn is_solid(self) -> bool {
        !self.is_air()
    }
}

/// A cubic section of the world: `SIZE`³ voxels in dense storage.
///
/// `SIZE` is 32 (design §10). Dense storage is `SIZE³ * 2` bytes = 64 KiB per
/// section — fine for M1's single chunk; M3 swaps the backing store for the
/// palette-compressed representation without changing this interface.
pub struct Section {
    blocks: Box<[BlockId]>,
}

impl Section {
    /// Edge length of a section in voxels.
    pub const SIZE: u32 = 32;
    /// Total voxels in a section.
    pub const VOLUME: usize = (Self::SIZE * Self::SIZE * Self::SIZE) as usize;

    /// A new section filled entirely with air.
    pub fn new() -> Self {
        Self {
            blocks: vec![BlockId::AIR; Self::VOLUME].into_boxed_slice(),
        }
    }

    /// Linear index for a local coordinate, laid out x-fastest then y then z.
    #[inline]
    fn index(x: u32, y: u32, z: u32) -> usize {
        debug_assert!(
            x < Self::SIZE && y < Self::SIZE && z < Self::SIZE,
            "({x}, {y}, {z}) out of bounds for a {}³ section",
            Self::SIZE
        );
        (x + Self::SIZE * (y + Self::SIZE * z)) as usize
    }

    /// The block at a local coordinate. Coordinates must be `< SIZE`.
    #[inline]
    pub fn get(&self, x: u32, y: u32, z: u32) -> BlockId {
        self.blocks[Self::index(x, y, z)]
    }

    /// Set the block at a local coordinate. Coordinates must be `< SIZE`.
    #[inline]
    pub fn set(&mut self, x: u32, y: u32, z: u32, block: BlockId) {
        let i = Self::index(x, y, z);
        self.blocks[i] = block;
    }

    /// Whether every voxel is air (nothing to mesh).
    pub fn is_empty(&self) -> bool {
        self.blocks.iter().all(|b| b.is_air())
    }
}

impl Default for Section {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STONE: BlockId = BlockId(1);
    const DIRT: BlockId = BlockId(2);

    #[test]
    fn air_is_the_default_and_is_not_solid() {
        assert!(BlockId::default().is_air());
        assert!(BlockId::AIR.is_air());
        assert!(!BlockId::AIR.is_solid());
        assert!(STONE.is_solid());
        assert!(!STONE.is_air());
    }

    #[test]
    fn a_fresh_section_is_all_air() {
        let s = Section::new();
        assert!(s.is_empty());
        assert_eq!(s.get(0, 0, 0), BlockId::AIR);
        assert_eq!(s.get(31, 31, 31), BlockId::AIR);
    }

    #[test]
    fn get_set_round_trips_including_the_corners() {
        let mut s = Section::new();
        for (x, y, z) in [
            (0, 0, 0),
            (31, 0, 0),
            (0, 31, 0),
            (0, 0, 31),
            (31, 31, 31),
            (5, 9, 17),
        ] {
            s.set(x, y, z, STONE);
            assert_eq!(s.get(x, y, z), STONE, "round-trip failed at ({x},{y},{z})");
        }
    }

    #[test]
    fn setting_one_voxel_leaves_the_rest_air() {
        let mut s = Section::new();
        s.set(5, 9, 17, DIRT);
        assert!(!s.is_empty());

        let mut solid = 0;
        for z in 0..Section::SIZE {
            for y in 0..Section::SIZE {
                for x in 0..Section::SIZE {
                    if s.get(x, y, z).is_solid() {
                        solid += 1;
                        assert_eq!((x, y, z), (5, 9, 17));
                    }
                }
            }
        }
        assert_eq!(solid, 1);
    }

    #[test]
    fn index_is_x_fastest_and_distinct_per_coordinate() {
        assert_eq!(Section::index(0, 0, 0), 0);
        assert_eq!(Section::index(1, 0, 0), 1);
        assert_eq!(Section::index(0, 1, 0), Section::SIZE as usize);
        assert_eq!(
            Section::index(0, 0, 1),
            (Section::SIZE * Section::SIZE) as usize
        );
        assert_eq!(Section::index(31, 31, 31), Section::VOLUME - 1);
    }
}
