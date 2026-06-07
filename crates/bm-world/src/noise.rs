//! Procedural-noise toolkit (M3): dependency-free, seeded **value noise** and friends.
//! Pure math, no block ids or world recipe — the *engine's* reusable primitives. The
//! terrain recipe that composes these into a specific world lives in the game (M9).
//! Deterministic so renders/golden-images reproduce.

/// Hash a 2D lattice point to `[0, 1)`.
pub fn hash(x: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32)
        .wrapping_mul(0x1657_4c2f)
        .wrapping_add((z as u32).wrapping_mul(0x68b3_8d2b))
        .wrapping_add(seed.wrapping_mul(0x9e37_79b9));
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    (h & 0x00ff_ffff) as f32 / 0x0100_0000 as f32
}

pub fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Bilinearly-interpolated value noise at `(x, z)`.
pub fn value_noise(x: f32, z: f32, seed: u32) -> f32 {
    let (xi, zi) = (x.floor() as i32, z.floor() as i32);
    let (xf, zf) = (x - xi as f32, z - zi as f32);
    let (u, v) = (smoothstep(xf), smoothstep(zf));
    let top = lerp(hash(xi, zi, seed), hash(xi + 1, zi, seed), u);
    let bot = lerp(hash(xi, zi + 1, seed), hash(xi + 1, zi + 1, seed), u);
    lerp(top, bot, v)
}

/// Fractal Brownian motion (a few octaves) → `[0, 1)`.
pub fn fbm(x: f32, z: f32, seed: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 1.0;
    let mut freq = 1.0;
    let mut norm = 0.0;
    for octave in 0..4 {
        sum += amp * value_noise(x * freq, z * freq, seed.wrapping_add(octave * 1013));
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// Hash a 3D lattice point to `[0, 1)` (for the cave noise).
pub fn hash3(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32)
        .wrapping_mul(0x1657_4c2f)
        .wrapping_add((y as u32).wrapping_mul(0x456b_2f1d))
        .wrapping_add((z as u32).wrapping_mul(0x68b3_8d2b))
        .wrapping_add(seed.wrapping_mul(0x9e37_79b9));
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    (h & 0x00ff_ffff) as f32 / 0x0100_0000 as f32
}

/// Trilinearly-interpolated 3D value noise.
pub fn value_noise3(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let (xi, yi, zi) = (x.floor() as i32, y.floor() as i32, z.floor() as i32);
    let (xf, yf, zf) = (x - xi as f32, y - yi as f32, z - zi as f32);
    let (u, v, w) = (smoothstep(xf), smoothstep(yf), smoothstep(zf));
    let c = |dx, dy, dz| hash3(xi + dx, yi + dy, zi + dz, seed);
    let x00 = lerp(c(0, 0, 0), c(1, 0, 0), u);
    let x10 = lerp(c(0, 1, 0), c(1, 1, 0), u);
    let x01 = lerp(c(0, 0, 1), c(1, 0, 1), u);
    let x11 = lerp(c(0, 1, 1), c(1, 1, 1), u);
    lerp(lerp(x00, x10, v), lerp(x01, x11, v), w)
}

/// 3D fractal noise (2 octaves) → `[0, 1)`, for carving caves.
pub fn fbm3(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let a = value_noise3(x, y, z, seed);
    let b = value_noise3(x * 2.1, y * 2.1, z * 2.1, seed.wrapping_add(8101));
    (a * 2.0 + b) / 3.0
}

/// Ridged noise → `[0, 1)` with sharp **ridge lines** (mountain crests) instead of
/// fbm's rounded hills: fold the noise around its midpoint and square the result.
pub fn ridged(x: f32, z: f32, seed: u32) -> f32 {
    let n = fbm(x, z, seed);
    let r = 1.0 - (2.0 * n - 1.0).abs();
    r * r
}

/// `smoothstep` remapped to an `[edge0, edge1]` range → `[0, 1]`.
pub fn smoothstep_range(edge0: f32, edge1: f32, x: f32) -> f32 {
    smoothstep(((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0))
}
