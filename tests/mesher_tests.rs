use bevy::prelude::*;
use bevy_symbios_ground::{HeightMapMeshBuilder, NormalMethod};
use symbios_ground::HeightMap;

fn flat_map(w: usize, h: usize, scale: f32) -> HeightMap {
    HeightMap::new(w, h, scale)
}

fn ramp_map(w: usize, h: usize, scale: f32) -> HeightMap {
    let mut map = HeightMap::new(w, h, scale);
    for z in 0..h {
        for x in 0..w {
            map.set(x, z, x as f32 * scale);
        }
    }
    map
}

#[test]
fn vertex_count_matches_dimensions() {
    let map = flat_map(8, 8, 1.0);
    let mesh = HeightMapMeshBuilder::new().build(&map);
    assert_eq!(mesh.count_vertices(), 8 * 8);
}

#[test]
fn index_count_matches_quads() {
    let map = flat_map(5, 7, 1.0);
    let mesh = HeightMapMeshBuilder::new().build(&map);
    // (w-1)*(h-1) quads × 6 indices each
    let expected = (5 - 1) * (7 - 1) * 6;
    assert_eq!(
        mesh.indices().expect("mesh must have indices").len(),
        expected
    );
}

#[test]
fn has_all_required_attributes() {
    let map = flat_map(4, 4, 1.0);
    let mesh = HeightMapMeshBuilder::new().build(&map);
    assert!(
        mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some(),
        "missing POSITION"
    );
    assert!(
        mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some(),
        "missing NORMAL"
    );
    assert!(
        mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some(),
        "missing UV_0"
    );
}

#[test]
fn flat_normals_point_up() {
    let map = flat_map(4, 4, 1.0);
    let mesh = HeightMapMeshBuilder::new().build(&map);
    let normals = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .expect("mesh must have normals")
        .as_float3()
        .expect("normals must be Float32x3");
    for n in normals {
        assert!(
            n[1] > 0.99,
            "flat terrain normal y should be ~1.0, got {:?}",
            n
        );
    }
}

#[test]
fn uv_attribute_has_correct_count() {
    let map = flat_map(5, 7, 1.0);
    let mesh = HeightMapMeshBuilder::new()
        .with_uv_tile_size(4.0)
        .build(&map);
    let uv_count = mesh
        .attribute(Mesh::ATTRIBUTE_UV_0)
        .expect("mesh must have UV_0")
        .len();
    assert_eq!(uv_count, 5 * 7);
}

#[test]
fn positions_encode_height_data() {
    let mut map = HeightMap::new(3, 3, 1.0);
    map.set(1, 1, 5.0);
    let mesh = HeightMapMeshBuilder::new().build(&map);

    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap();

    // Vertex at (x=1, z=1) is index z*w+x = 1*3+1 = 4
    let center = positions[4];
    assert_eq!(center[0], 1.0, "world_x = 1 * scale(1.0)");
    assert_eq!(center[1], 5.0, "height at (1,1)");
    assert_eq!(center[2], 1.0, "world_z = 1 * scale(1.0)");
}

#[test]
fn positions_origin_is_zero() {
    let map = flat_map(4, 4, 2.0);
    let mesh = HeightMapMeshBuilder::new().build(&map);
    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap();
    assert_eq!(positions[0], [0.0, 0.0, 0.0]);
}

#[test]
fn positions_far_corner_matches_scale() {
    // 4×4 grid with scale 2.0 → far corner at (3*2, 0, 3*2) = (6, 0, 6)
    let map = flat_map(4, 4, 2.0);
    let mesh = HeightMapMeshBuilder::new().build(&map);
    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap();
    let last = *positions.last().unwrap();
    assert_eq!(last[0], 6.0, "far corner x");
    assert_eq!(last[2], 6.0, "far corner z");
}

#[test]
#[should_panic]
fn panics_on_1x1_map() {
    let map = flat_map(1, 1, 1.0);
    HeightMapMeshBuilder::new().build(&map);
}

#[test]
fn sobel_flat_normals_point_up() {
    let map = flat_map(4, 4, 1.0);
    let mesh = HeightMapMeshBuilder::new()
        .with_normal_method(NormalMethod::Sobel)
        .build(&map);
    let normals = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .expect("mesh must have normals")
        .as_float3()
        .expect("normals must be Float32x3");
    for n in normals {
        assert!(
            n[1] > 0.99,
            "Sobel flat terrain normal y should be ~1.0, got {:?}",
            n
        );
    }
}

#[test]
fn sobel_ramp_normals_have_x_component() {
    let map = ramp_map(8, 8, 1.0);
    let mesh = HeightMapMeshBuilder::new()
        .with_normal_method(NormalMethod::Sobel)
        .build(&map);
    let normals = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .unwrap()
        .as_float3()
        .unwrap();
    // Interior vertex on an X-slope must have a non-zero X normal component.
    let interior = normals[1 * 8 + 4]; // z=1, x=4
    assert!(
        interior[0].abs() > 0.01,
        "Sobel ramp normal should have X component, got {:?}",
        interior
    );
}

#[test]
fn sobel_normal_is_unit_length() {
    let map = ramp_map(6, 6, 2.0);
    let mesh = HeightMapMeshBuilder::new()
        .with_normal_method(NormalMethod::Sobel)
        .build(&map);
    let normals = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .unwrap()
        .as_float3()
        .unwrap();
    for n in normals {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert!(
            (len - 1.0).abs() < 1e-5,
            "Sobel normal should be unit length, got length {len} for {:?}",
            n
        );
    }
}

#[test]
fn ramp_normals_have_x_component() {
    let map = ramp_map(8, 8, 1.0);
    let mesh = HeightMapMeshBuilder::new().build(&map);
    let normals = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .unwrap()
        .as_float3()
        .unwrap();
    // Interior vertices on a slope along X must have a non-zero X normal component
    let interior = normals[1 * 8 + 4]; // z=1, x=4
    assert!(
        interior[0].abs() > 0.01,
        "ramp normal should have X component, got {:?}",
        interior
    );
}
