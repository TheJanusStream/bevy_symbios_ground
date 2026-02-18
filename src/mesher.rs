//! Mesh generation from `HeightMap` data.
//!
//! Converts a [`HeightMap`] into a Bevy [`Mesh`] with:
//! - `TriangleList` topology
//! - Smooth per-vertex normals (area-weighted average of adjacent face normals)
//! - Tiling UV coordinates (world-space scaled by `uv_tile_size`)

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use symbios_ground::HeightMap;

/// Converts a [`HeightMap`] into a Bevy [`Mesh`].
///
/// The mesh covers world space `[0, world_width] × [0, world_depth]` in the XZ
/// plane, with heights along the Y axis. Per-vertex normals are computed from
/// the actual mesh geometry: each triangle's unnormalized cross-product (which
/// is proportional to its area) is accumulated at each of its three vertices,
/// then normalized. This area-weighted averaging gives accurate shading for
/// jagged or eroded terrain where the central-difference approximation diverges
/// from the rendered surface.
///
/// # UV Mapping
///
/// UVs are world-space coordinates divided by `uv_tile_size`:
/// `u = world_x / uv_tile_size`, `v = world_z / uv_tile_size`.
///
/// Setting `uv_tile_size = scale` tiles the texture once per grid cell.
/// Setting `uv_tile_size = world_width` stretches the texture over the whole mesh.
///
/// # Example
///
/// ```ignore
/// use bevy_symbios_ground::HeightMapMeshBuilder;
/// use symbios_ground::HeightMap;
///
/// let heightmap = HeightMap::new(64, 64, 1.0);
/// let mesh = HeightMapMeshBuilder::new()
///     .with_uv_tile_size(4.0)
///     .build(&heightmap);
/// ```
pub struct HeightMapMeshBuilder {
    uv_tile_size: f32,
}

impl Default for HeightMapMeshBuilder {
    fn default() -> Self {
        Self { uv_tile_size: 1.0 }
    }
}

impl HeightMapMeshBuilder {
    /// Creates a new builder with default settings (`uv_tile_size = 1.0`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the world-space size of one UV tile.
    ///
    /// A value of `1.0` tiles the texture once per world unit.
    /// A value equal to `heightmap.scale` tiles once per grid cell.
    /// Clamped to a positive minimum to avoid division by zero.
    pub fn with_uv_tile_size(mut self, size: f32) -> Self {
        self.uv_tile_size = size.max(f32::EPSILON);
        self
    }

    /// Builds the mesh from the given heightmap, consuming the builder.
    ///
    /// Produces a `TriangleList` mesh with positions, normals, and UV_0.
    ///
    /// # Panics
    ///
    /// Panics if the heightmap dimensions are less than 2×2, as at least one
    /// quad is required to produce valid triangle geometry.
    pub fn build(&self, heightmap: &HeightMap) -> Mesh {
        assert!(
            heightmap.width() >= 2 && heightmap.height() >= 2,
            "HeightMap must be at least 2×2 to generate a mesh (got {}×{})",
            heightmap.width(),
            heightmap.height()
        );

        let w = heightmap.width();
        let h = heightmap.height();
        let s = heightmap.scale();

        let vertex_count = w * h;
        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(vertex_count);

        for z in 0..h {
            for x in 0..w {
                let world_x = x as f32 * s;
                let world_z = z as f32 * s;
                let world_y = heightmap.get(x, z);

                positions.push([world_x, world_y, world_z]);
                uvs.push([world_x / self.uv_tile_size, world_z / self.uv_tile_size]);
            }
        }

        // Build CCW triangle indices (normal pointing +Y when terrain is flat).
        // Each quad (x, z) → (x+1, z+1) emits two triangles:
        //   tl──tr
        //   │╲  │     Triangle 1: tl, bl, tr
        //   │ ╲ │     Triangle 2: tr, bl, br
        //   bl──br
        let quad_count = (w - 1) * (h - 1);
        let mut indices: Vec<u32> = Vec::with_capacity(quad_count * 6);

        for z in 0..(h - 1) {
            for x in 0..(w - 1) {
                let tl = (z * w + x) as u32;
                let tr = (z * w + x + 1) as u32;
                let bl = ((z + 1) * w + x) as u32;
                let br = ((z + 1) * w + x + 1) as u32;

                // Triangle 1 — CCW: cross(bl-tl, tr-tl) = +Y for flat terrain
                indices.push(tl);
                indices.push(bl);
                indices.push(tr);

                // Triangle 2 — CCW: cross(bl-tr, br-tr) = +Y for flat terrain
                indices.push(tr);
                indices.push(bl);
                indices.push(br);
            }
        }

        // Compute smooth per-vertex normals from the actual mesh geometry.
        //
        // For each triangle, accumulate the unnormalized face normal (cross
        // product of two edges) at each of its three vertices.  The cross
        // product magnitude equals twice the triangle's area, so larger
        // triangles automatically contribute more to the averaged normal
        // (area weighting).  Finally, normalize each accumulated vector.
        //
        // This gives better results than the central-difference formula from
        // `HeightMap::get_normal_at` when the terrain is jagged or has been
        // modified by erosion, because it reflects the actual rendered triangles
        // rather than a continuous approximation of the underlying function.
        let mut normals: Vec<Vec3> = vec![Vec3::ZERO; vertex_count];

        for tri in indices.chunks_exact(3) {
            let [i0, i1, i2] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
            let p0 = Vec3::from(positions[i0]);
            let p1 = Vec3::from(positions[i1]);
            let p2 = Vec3::from(positions[i2]);
            let face_normal = (p1 - p0).cross(p2 - p0);
            normals[i0] += face_normal;
            normals[i1] += face_normal;
            normals[i2] += face_normal;
        }

        let normals: Vec<[f32; 3]> = normals
            .iter()
            .map(|n| {
                let len = n.length();
                // Degenerate vertex (zero contributions): default to +Y.
                if len > f32::EPSILON { (*n / len).into() } else { [0.0, 1.0, 0.0] }
            })
            .collect();

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_indices(Indices::U32(indices));
        mesh
    }
}
