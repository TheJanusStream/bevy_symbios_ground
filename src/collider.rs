//! Avian3D physics collider generation from `HeightMap` data.
//!
//! Provides [`build_heightfield_collider`] which converts a [`HeightMap`] into
//! an Avian3D `Collider::heightfield`. This is the most efficient collision
//! shape for static terrain — far cheaper than `trimesh` for ray-casting and
//! contact queries.

use avian3d::prelude::Collider;
use bevy::prelude::*;
use symbios_ground::HeightMap;

/// Builds an Avian3D `Collider::heightfield` from a [`HeightMap`].
///
/// The collider is centered at the origin of its local space, spanning
/// `[-world_width/2, world_width/2]` × `[-world_depth/2, world_depth/2]`
/// in the XZ plane. Heights are in world units.
///
/// Attach the returned `Collider` to the same entity as your terrain mesh.
/// Because the heightfield is centered at origin while the mesh starts at
/// `(0, 0, 0)`, offset the entity's `Transform` by
/// `(-world_width/2, 0, -world_depth/2)` if you want them to align.
///
/// # Panics
///
/// Panics if the heightmap has zero width or height (upheld by [`HeightMap::new`]).
///
/// # Example
///
/// ```ignore
/// use bevy_symbios_ground::build_heightfield_collider;
/// use symbios_ground::HeightMap;
///
/// let heightmap = HeightMap::new(64, 64, 1.0);
/// let collider = build_heightfield_collider(&heightmap);
/// // commands.spawn((collider, ...));
/// ```
pub fn build_heightfield_collider(heightmap: &HeightMap) -> Collider {
    let w = heightmap.width;
    let h = heightmap.height;

    // Avian's 3D heightfield expects `heights[row][col]` where:
    //   rows  → subdivisions along X axis (width)
    //   cols  → subdivisions along Z axis (height)
    // HeightMap stores data[z * width + x], so we transpose accordingly.
    let heights: Vec<Vec<f32>> = (0..w)
        .map(|x| (0..h).map(|z| heightmap.get(x, z)).collect())
        .collect();

    // `scale` is the total world extent of the heightfield on each axis.
    // Y scale = 1.0 because heights are already in world units.
    let scale = Vec3::new(heightmap.world_width(), 1.0, heightmap.world_depth());

    Collider::heightfield(heights, scale)
}
