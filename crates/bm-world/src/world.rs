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

/// A cubic section of the world: `SIZE`³ voxels, **palette-compressed** (design
/// §7.3). A small palette of distinct blocks + bit-packed indices
/// (`ceil(log2(palette))` bits per voxel), so a low-diversity chunk costs a few KiB
/// instead of 64 KiB. The `get`/`set` API is unchanged from the old dense store.
///
/// The palette only grows (no shrink/repack on removal yet); indices widen when the
/// palette outgrows the current bit width.
pub struct Section {
    palette: Vec<BlockId>,
    /// Bit-packed palette indices, `bits` per voxel, x-fastest.
    indices: Vec<u64>,
    bits: u32,
}

impl Section {
    /// Edge length of a section in voxels.
    pub const SIZE: u32 = 32;
    /// Total voxels in a section.
    pub const VOLUME: usize = (Self::SIZE * Self::SIZE * Self::SIZE) as usize;

    /// A new section filled entirely with air. **Uniform fast path (M11):** a one-entry
    /// palette stores **zero bits per voxel** — no index array at all — so an all-air (or
    /// any uniform) section costs ~8 bytes instead of 4 KiB, and `uniform()` lets the mesher
    /// skip it outright. Indices materialise on the first write of a second block.
    pub fn new() -> Self {
        Self {
            palette: vec![BlockId::AIR],
            indices: Vec::new(),
            bits: 0,
        }
    }

    /// `Some(block)` when the whole section is provably one block (a one-entry palette —
    /// the M11 uniform fast path). `None` means "mixed or unknown" (the palette only grows,
    /// so a section written then overwritten back to uniform reports `None`; that's fine —
    /// this is an optimisation hint, never a correctness gate).
    pub fn uniform(&self) -> Option<BlockId> {
        (self.palette.len() == 1).then(|| self.palette[0])
    }

    /// `u64` words needed to hold `VOLUME` indices of `bits` each.
    fn words(bits: u32) -> usize {
        (Self::VOLUME * bits as usize).div_ceil(64)
    }

    /// Minimum bits to index a palette of `len` entries (at least 1).
    fn bits_for(len: usize) -> u32 {
        (usize::BITS - (len.max(2) - 1).leading_zeros()).max(1)
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

    /// Read the `bits`-wide value at slot `i` from a packed buffer (may span words).
    #[inline]
    fn read_bits(buf: &[u64], i: usize, bits: u32) -> usize {
        let bits = bits as usize;
        let bit = i * bits;
        let (word, off) = (bit / 64, bit % 64);
        let mask = (1u64 << bits) - 1;
        let mut v = buf[word] >> off;
        if off + bits > 64 {
            v |= buf[word + 1] << (64 - off);
        }
        (v & mask) as usize
    }

    /// Write a `bits`-wide `val` into slot `i` of a packed buffer (may span words).
    #[inline]
    fn write_bits(buf: &mut [u64], i: usize, bits: u32, val: usize) {
        let bits = bits as usize;
        let bit = i * bits;
        let (word, off) = (bit / 64, bit % 64);
        let mask = (1u64 << bits) - 1;
        let val = val as u64 & mask;
        buf[word] = (buf[word] & !(mask << off)) | (val << off);
        if off + bits > 64 {
            let rem = 64 - off;
            buf[word + 1] = (buf[word + 1] & !(mask >> rem)) | (val >> rem);
        }
    }

    /// Find `block` in the palette, adding it (and widening indices) if new.
    fn palette_index(&mut self, block: BlockId) -> usize {
        if let Some(p) = self.palette.iter().position(|&b| b == block) {
            return p;
        }
        self.palette.push(block);
        let needed = Self::bits_for(self.palette.len());
        if needed > self.bits {
            // Widen (incl. the 0-bit uniform case: every implicit index was 0, so a fresh
            // zeroed buffer is already correct and the copy loop is skipped).
            let mut wider = vec![0u64; Self::words(needed)];
            if self.bits > 0 {
                for i in 0..Self::VOLUME {
                    let idx = Self::read_bits(&self.indices, i, self.bits);
                    Self::write_bits(&mut wider, i, needed, idx);
                }
            }
            self.indices = wider;
            self.bits = needed;
        }
        self.palette.len() - 1
    }

    /// The block at a local coordinate. Coordinates must be `< SIZE`.
    #[inline]
    pub fn get(&self, x: u32, y: u32, z: u32) -> BlockId {
        if self.bits == 0 {
            debug_assert!(x < Self::SIZE && y < Self::SIZE && z < Self::SIZE);
            return self.palette[0]; // uniform fast path (M11)
        }
        let idx = Self::read_bits(&self.indices, Self::index(x, y, z), self.bits);
        self.palette[idx]
    }

    /// Set the block at a local coordinate. Coordinates must be `< SIZE`.
    #[inline]
    pub fn set(&mut self, x: u32, y: u32, z: u32, block: BlockId) {
        let p = self.palette_index(block);
        if self.bits == 0 {
            debug_assert!(x < Self::SIZE && y < Self::SIZE && z < Self::SIZE);
            return; // uniform write of the same block: nothing to store (M11)
        }
        Self::write_bits(&mut self.indices, Self::index(x, y, z), self.bits, p);
    }

    /// Whether every voxel is air (nothing to mesh).
    pub fn is_empty(&self) -> bool {
        if let Some(b) = self.uniform() {
            return b.is_air(); // uniform fast path (M11)
        }
        (0..Self::VOLUME)
            .all(|i| self.palette[Self::read_bits(&self.indices, i, self.bits)].is_air())
    }

    /// Approximate heap bytes used by this section's storage (for memory tests).
    pub fn mem_bytes(&self) -> usize {
        self.palette.len() * std::mem::size_of::<BlockId>() + self.indices.len() * 8
    }
}

impl Default for Section {
    fn default() -> Self {
        Self::new()
    }
}

/// Chunk coordinate: which section, measured in whole sections. World voxel origin
/// of a chunk is `coord * SIZE`.
pub type ChunkCoord = (i32, i32, i32);

/// A sparse grid of sections keyed by [`ChunkCoord`]. For M2 this is a small,
/// hand-populated world; M3 adds palette storage, generation, and streaming behind
/// the same accessors. Stays free of meshing/GPU types (architecture §4).
#[derive(Default)]
pub struct World {
    sections: std::collections::HashMap<ChunkCoord, Section>,
}

impl World {
    pub fn new() -> Self {
        World::default()
    }

    pub fn insert(&mut self, coord: ChunkCoord, section: Section) {
        self.sections.insert(coord, section);
    }

    /// The section at `coord`, if present. Returns `Option<&Section>` so it drops
    /// straight into the mesher's neighbour slots.
    pub fn get(&self, coord: ChunkCoord) -> Option<&Section> {
        self.sections.get(&coord)
    }

    /// Drop a section (streaming evicts distant chunks to bound memory).
    pub fn remove(&mut self, coord: ChunkCoord) -> Option<Section> {
        self.sections.remove(&coord)
    }

    /// Whether a section is present at `coord`.
    pub fn contains(&self, coord: ChunkCoord) -> bool {
        self.sections.contains_key(&coord)
    }

    pub fn chunks(&self) -> impl Iterator<Item = (ChunkCoord, &Section)> {
        self.sections.iter().map(|(&c, s)| (c, s))
    }

    pub fn len(&self) -> usize {
        self.sections.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn uniform_fast_path_zero_bits_and_round_trip() {
        // M11: a fresh (all-air) section stores no index words and reports uniform.
        let mut s = Section::new();
        assert_eq!(s.uniform(), Some(BlockId::AIR));
        assert_eq!(s.mem_bytes(), std::mem::size_of::<BlockId>()); // ~2 bytes, not 4 KiB
        assert!(s.is_empty());
        assert_eq!(s.get(31, 31, 31), BlockId::AIR);
        // Writing the same (uniform) block stays 0-bit.
        s.set(5, 5, 5, BlockId::AIR);
        assert_eq!(s.uniform(), Some(BlockId::AIR));
        assert!(s.mem_bytes() < 64);
        // The first different block materialises indices and round-trips exactly.
        s.set(1, 2, 3, BlockId(7));
        assert_eq!(s.uniform(), None);
        assert_eq!(s.get(1, 2, 3), BlockId(7));
        assert_eq!(s.get(0, 0, 0), BlockId::AIR);
        assert!(s.mem_bytes() >= Section::VOLUME / 8); // 1 bit/voxel materialised
    }

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

    #[test]
    fn world_stores_and_returns_sections_by_coord() {
        let mut world = World::new();
        assert!(world.is_empty());
        let mut s = Section::new();
        s.set(1, 2, 3, BlockId(1));
        world.insert((0, 0, 0), s);
        world.insert((1, 0, 0), Section::new());

        assert_eq!(world.len(), 2);
        assert_eq!(world.get((0, 0, 0)).unwrap().get(1, 2, 3), BlockId(1));
        assert!(world.get((1, 0, 0)).is_some());
        assert!(world.get((9, 9, 9)).is_none());
        assert_eq!(world.chunks().count(), 2);
    }

    #[test]
    fn palette_widens_and_round_trips_many_materials() {
        let mut s = Section::new();
        // 16 distinct materials along a row -> palette of 17 (incl AIR) -> 5 bits.
        for i in 0..16u32 {
            s.set(i, 0, 0, BlockId((i + 1) as u16));
        }
        for i in 0..16u32 {
            assert_eq!(s.get(i, 0, 0), BlockId((i + 1) as u16));
        }
        // Untouched voxels are still air.
        assert_eq!(s.get(31, 31, 31), BlockId::AIR);
        // Still far smaller than the 64 KiB dense store.
        assert!(s.mem_bytes() < Section::VOLUME * 2);
    }

    #[test]
    fn low_diversity_section_is_compact() {
        let mut s = Section::new();
        for x in 0..Section::SIZE {
            s.set(x, 0, 0, BlockId(1));
        }
        // Palette {air, stone} -> 1 bit/voxel -> ~4 KiB, vs 64 KiB dense.
        assert!(s.mem_bytes() < 8 * 1024, "got {} bytes", s.mem_bytes());
    }

    #[test]
    fn overwriting_back_to_air_reads_as_air() {
        let mut s = Section::new();
        s.set(3, 4, 5, BlockId(2));
        assert_eq!(s.get(3, 4, 5), BlockId(2));
        s.set(3, 4, 5, BlockId::AIR);
        assert_eq!(s.get(3, 4, 5), BlockId::AIR);
        assert!(s.is_empty());
    }
}
