//! SplatMap (weight map) to GPU texture conversion and sync.
//!
//! Provides utilities to convert a [`WeightMap`] from `symbios-ground` into a
//! Bevy [`Image`] (RGBA8 GPU texture), and a Bevy system to keep the texture
//! in sync when terrain data changes.

use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use symbios_ground::WeightMap;

/// Converts a [`WeightMap`] into a tiling Bevy [`Image`] (RGBA8Unorm).
///
/// Each pixel maps directly: R = layer 0 weight, G = layer 1, B = layer 2, A = layer 3.
/// The image uses `Repeat` address mode for tiling in shaders.
///
/// # Example
///
/// ```ignore
/// use bevy_symbios_ground::splat_to_image;
/// use symbios_ground::{HeightMap, SplatMapper};
///
/// let heightmap = HeightMap::new(64, 64, 1.0);
/// let weight_map = SplatMapper::default().generate(&heightmap);
/// let image = splat_to_image(&weight_map);
/// ```
pub fn splat_to_image(weight_map: &WeightMap) -> Image {
    // Flatten [u8; 4] pixel data into a raw byte buffer
    let raw: Vec<u8> = weight_map
        .data
        .iter()
        .flat_map(|pixel| pixel.iter().copied())
        .collect();

    let mut image = Image::new(
        Extent3d {
            width: weight_map.width as u32,
            height: weight_map.height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        raw,
        TextureFormat::Rgba8Unorm,
        default(),
    );

    // Repeat addressing so the splatmap tiles seamlessly with world-space UVs
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        ..default()
    });

    image
}

/// Resource holding the current [`WeightMap`] and whether it has changed.
///
/// Mutate `weight_map` and call [`mark_dirty`] to trigger the next
/// [`sync_splat_texture`] pass to re-upload the GPU texture.
///
/// [`mark_dirty`]: GroundMaterialSettings::mark_dirty
#[derive(Resource)]
pub struct GroundMaterialSettings {
    /// The current weight map data. Replace or modify to update terrain appearance.
    pub weight_map: WeightMap,
    dirty: bool,
}

impl GroundMaterialSettings {
    /// Creates a new settings resource from a weight map.
    /// The texture will be uploaded on the next [`sync_splat_texture`] run.
    pub fn new(weight_map: WeightMap) -> Self {
        Self {
            weight_map,
            dirty: true,
        }
    }

    /// Marks the weight map as changed so [`sync_splat_texture`] re-uploads it.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

/// Resource holding the GPU-side splat texture handle.
///
/// Insert this alongside [`GroundMaterialSettings`] before running
/// [`sync_splat_texture`]. The handle can be used as a `base_color_texture`
/// on a `StandardMaterial` or a custom terrain material.
///
/// # Example
///
/// ```ignore
/// commands.insert_resource(SplatTexture { handle: images.add(initial_image) });
/// commands.insert_resource(GroundMaterialSettings::new(weight_map));
/// app.add_systems(Update, sync_splat_texture);
/// ```
#[derive(Resource)]
pub struct SplatTexture {
    /// Handle to the GPU texture. Pass to your material's texture slot.
    pub handle: Handle<Image>,
}

/// Bevy system that re-uploads the splat texture when [`GroundMaterialSettings`]
/// is marked dirty.
///
/// Add to your `Update` schedule. Only re-uploads when data has changed,
/// so it is safe to run every frame.
pub fn sync_splat_texture(
    mut settings: ResMut<GroundMaterialSettings>,
    splat_texture: Res<SplatTexture>,
    mut images: ResMut<Assets<Image>>,
) {
    if !settings.dirty {
        return;
    }
    settings.dirty = false;

    let Some(image) = images.get_mut(&splat_texture.handle) else {
        return;
    };

    let weight_map = &settings.weight_map;

    // Resize texture data in-place if dimensions changed
    let expected_bytes = weight_map.width * weight_map.height * 4;
    if image.data.as_ref().map(|d| d.len()).unwrap_or(0) != expected_bytes {
        image.texture_descriptor.size = Extent3d {
            width: weight_map.width as u32,
            height: weight_map.height as u32,
            depth_or_array_layers: 1,
        };
    }

    let raw: Vec<u8> = weight_map
        .data
        .iter()
        .flat_map(|pixel| pixel.iter().copied())
        .collect();

    image.data = Some(raw);
}
