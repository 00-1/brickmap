//! Meshing throughput (M2). Records greedy vs naïve mesher cost on a terrain-like
//! section, tracked against the design §8 budget. Run with `cargo bench`.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use brickmap::mesh::{greedy_mesh_section, mesh_section};
use brickmap::world::{BlockId, Section};

/// A representative half-filled section: a rolling heightmap with a grass surface
/// over stone — closer to real chunks than a solid cube.
fn terrain_section() -> Section {
    let mut s = Section::new();
    let n = Section::SIZE;
    for z in 0..n {
        for x in 0..n {
            let h = (16.0 + 7.0 * ((x as f32 * 0.30).sin() + (z as f32 * 0.27).cos())) as u32;
            let h = h.clamp(1, n - 1);
            for y in 0..h {
                s.set(x, y, z, if y + 1 == h { BlockId(3) } else { BlockId(1) });
            }
        }
    }
    s
}

fn meshing(c: &mut Criterion) {
    let section = terrain_section();
    let mut group = c.benchmark_group("mesh_terrain_section");
    group.bench_function("greedy", |b| {
        b.iter(|| greedy_mesh_section(black_box(&section)))
    });
    group.bench_function("naive", |b| b.iter(|| mesh_section(black_box(&section))));
    group.finish();
}

criterion_group!(benches, meshing);
criterion_main!(benches);
