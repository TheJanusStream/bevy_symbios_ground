use std::hint::black_box;

use bevy_symbios_ground::HeightMapMeshBuilder;
use criterion::{Criterion, criterion_group, criterion_main};
use symbios_ground::HeightMap;

fn bench_mesh_generation(c: &mut Criterion) {
    let mut map = HeightMap::new(128, 128, 1.0);
    for z in 0..128 {
        for x in 0..128 {
            map.set(x, z, ((x + z) as f32 * 0.1).sin());
        }
    }

    c.bench_function("HeightMapMeshBuilder 128x128", |b| {
        b.iter(|| {
            HeightMapMeshBuilder::new()
                .with_uv_tile_size(4.0)
                .build(black_box(&map))
        });
    });
}

criterion_group!(benches, bench_mesh_generation);
criterion_main!(benches);
