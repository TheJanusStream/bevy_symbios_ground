use bevy_symbios_ground::splat_to_image;
use symbios_ground::{HeightMap, SplatMapper, WeightMap};

fn make_weight_map(w: usize, h: usize) -> WeightMap {
    let mut wm = WeightMap::new(w, h);
    // Set recognizable pixel patterns for assertion
    for i in 0..w * h {
        wm.data[i] = [
            (i % 256) as u8,
            ((i + 64) % 256) as u8,
            ((i + 128) % 256) as u8,
            ((i + 192) % 256) as u8,
        ];
    }
    wm
}

#[test]
fn image_dimensions_match_weight_map() {
    let wm = make_weight_map(16, 32);
    let image = splat_to_image(&wm);
    assert_eq!(image.texture_descriptor.size.width, 16);
    assert_eq!(image.texture_descriptor.size.height, 32);
}

#[test]
fn image_data_length_is_four_bytes_per_pixel() {
    let wm = make_weight_map(8, 8);
    let image = splat_to_image(&wm);
    assert_eq!(image.data.as_ref().map(|d| d.len()).unwrap_or(0), 8 * 8 * 4);
}

#[test]
fn pixel_data_round_trips_correctly() {
    let mut wm = WeightMap::new(4, 4);
    wm.data[0] = [10, 20, 30, 40];
    wm.data[5] = [100, 150, 200, 250];

    let image = splat_to_image(&wm);
    let data = image.data.as_ref().expect("image must have data");

    // Pixel 0
    assert_eq!(data[0], 10);
    assert_eq!(data[1], 20);
    assert_eq!(data[2], 30);
    assert_eq!(data[3], 40);

    // Pixel 5
    let offset = 5 * 4;
    assert_eq!(data[offset], 100);
    assert_eq!(data[offset + 1], 150);
    assert_eq!(data[offset + 2], 200);
    assert_eq!(data[offset + 3], 250);
}

#[test]
fn splat_mapper_output_converts_without_panic() {
    let mut heightmap = HeightMap::new(32, 32, 1.0);
    for z in 0..32 {
        for x in 0..32 {
            heightmap.set(x, z, (x as f32 / 31.0 + z as f32 / 31.0) * 0.5);
        }
    }
    let weight_map = SplatMapper::default().generate(&heightmap);
    let image = splat_to_image(&weight_map);
    assert_eq!(image.texture_descriptor.size.width, 32);
    assert_eq!(image.texture_descriptor.size.height, 32);
}

#[test]
fn rgba8_unorm_format() {
    use bevy::render::render_resource::TextureFormat;
    let wm = make_weight_map(4, 4);
    let image = splat_to_image(&wm);
    assert_eq!(image.texture_descriptor.format, TextureFormat::Rgba8Unorm);
}
