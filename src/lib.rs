//! Bevy integration for `symbios-ground` terrain data.
//!
//! Translates [`HeightMap`] and [`WeightMap`] data from the `symbios-ground` crate
//! into Bevy-compatible meshes, physics colliders, and GPU textures.
//!
//! # Features
//!
//! - **Mesh generation**: Convert a `HeightMap` to a Bevy [`Mesh`] with correct
//!   topology, smooth normals, and tiling UV coordinates via [`HeightMapMeshBuilder`].
//! - **Splat textures**: Convert a `WeightMap` to a Bevy [`Image`] (RGBA8 GPU texture)
//!   for use with terrain shaders via [`splat`].
//! - **Physics colliders** (optional, `physics` feature): Generate an Avian3D
//!   `Collider::heightfield` from a `HeightMap` via [`collider`].
//!
//! # Feature Flags
//!
//! - `physics`: Enables [`collider`] and [`collider::build_heightfield_collider`]
//!   for Avian3D integration.
//!
//! # Example
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_symbios_ground::HeightMapMeshBuilder;
//! use symbios_ground::{HeightMap, generators::FbmNoise, TerrainGenerator};
//!
//! fn setup(
//!     mut commands: Commands,
//!     mut meshes: ResMut<Assets<Mesh>>,
//!     mut materials: ResMut<Assets<StandardMaterial>>,
//! ) {
//!     let mut heightmap = HeightMap::new(128, 128, 1.0);
//!     FbmNoise::default().generate(&mut heightmap);
//!     heightmap.normalize();
//!
//!     let mesh = HeightMapMeshBuilder::new()
//!         .with_uv_tile_size(4.0)
//!         .build(&heightmap);
//!
//!     commands.spawn((
//!         Mesh3d(meshes.add(mesh)),
//!         MeshMaterial3d(materials.add(StandardMaterial::default())),
//!     ));
//! }
//! ```

pub mod mesher;
pub mod splat;

#[cfg(feature = "physics")]
pub mod collider;

pub use mesher::{HeightMapMeshBuilder, NormalMethod};
pub use splat::{GroundMaterialSettings, SplatTexture, splat_to_image, sync_splat_texture};

#[cfg(feature = "physics")]
pub use collider::build_heightfield_collider;
