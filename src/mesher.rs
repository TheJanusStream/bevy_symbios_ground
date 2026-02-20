//! Mesh generation from `HeightMap` data.
//!
//! Converts a [`HeightMap`] into a Bevy [`Mesh`] with:
//! - `TriangleList` topology
//! - Smooth per-vertex normals (area-weighted average of adjacent face normals,
//!   or Sobel filter applied directly to the heightmap)
//! - Tiling UV coordinates (world-space scaled by `uv_tile_size`)

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use symbios_ground::HeightMap;

/// Selects the algorithm used to compute per-vertex normals in [`HeightMapMeshBuilder`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NormalMethod {
    /// Area-weighted average of adjacent triangle face normals (default).
    ///
    /// Accumulates the unnormalized cross-product of each triangle (whose
    /// magnitude equals twice the triangle area) at its three vertices, then
    /// normalizes. Accurately reflects the rendered triangles, making it the
    /// better choice for jagged or eroded terrain.
    #[default]
    AreaWeighted,

    /// Sobel filter applied directly to the heightmap grid.
    ///
    /// Computes the height gradient at each vertex using the 3×3 Sobel kernels,
    /// then derives the surface normal analytically. Produces smoother, more
    /// continuous normals and is faster than `AreaWeighted` because it avoids
    /// the triangle-accumulation pass. Best suited for smooth procedural
    /// terrain where the continuous approximation is valid.
    Sobel,
}

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
    normal_method: NormalMethod,
}

impl Default for HeightMapMeshBuilder {
    fn default() -> Self {
        Self {
            uv_tile_size: 1.0,
            normal_method: NormalMethod::default(),
        }
    }
}

impl HeightMapMeshBuilder {
    /// Creates a new builder with default settings (`uv_tile_size = 1.0`,
    /// `normal_method = AreaWeighted`).
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

    /// Selects the algorithm used to compute per-vertex normals.
    ///
    /// See [`NormalMethod`] for a description of each variant.
    pub fn with_normal_method(mut self, method: NormalMethod) -> Self {
        self.normal_method = method;
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

        let normals: Vec<[f32; 3]> = match self.normal_method {
            NormalMethod::AreaWeighted => {
                // Accumulate unnormalized face normals (cross products) at each
                // vertex. The cross-product magnitude equals twice the triangle
                // area, so larger triangles contribute proportionally more
                // (area weighting). Reflects the actual rendered geometry.
                let mut acc: Vec<Vec3> = vec![Vec3::ZERO; vertex_count];
                for tri in indices.chunks_exact(3) {
                    let [i0, i1, i2] =
                        [tri[0] as usize, tri[1] as usize, tri[2] as usize];
                    let p0 = Vec3::from(positions[i0]);
                    let p1 = Vec3::from(positions[i1]);
                    let p2 = Vec3::from(positions[i2]);
                    let face_normal = (p1 - p0).cross(p2 - p0);
                    acc[i0] += face_normal;
                    acc[i1] += face_normal;
                    acc[i2] += face_normal;
                }
                acc.iter()
                    .map(|n| {
                        let len = n.length();
                        if len > f32::EPSILON {
                            (*n / len).into()
                        } else {
                            [0.0, 1.0, 0.0]
                        }
                    })
                    .collect()
            }
            NormalMethod::Sobel => compute_normals_sobel(heightmap),
        };

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

/// Computes per-vertex normals using a 3×3 Sobel filter over the heightmap.
///
/// For each grid vertex `(xi, zi)`, the 3×3 neighborhood of heights is sampled
/// (edge vertices clamp to the nearest valid index). The Sobel X kernel
/// `[[-1,0,1],[-2,0,2],[-1,0,1]]` and Sobel Z kernel
/// `[[-1,-2,-1],[0,0,0],[1,2,1]]` produce weighted height gradients `gx` and
/// `gz`. The surface normal follows from the cross product of the two tangent
/// vectors:
///
/// ```text
/// normal ∝ (-gx, 8·scale, -gz)
/// ```
///
/// where `scale` is the world-space grid spacing. The factor `8·scale` arises
/// because the Sobel kernels approximate the derivative as `dh/dx ≈ gx/(8s)`,
/// so the unnormalized normal `(-dh/dx, 1, -dh/dz)` scaled by `8s` becomes
/// `(-gx, 8s, -gz)`.
fn compute_normals_sobel(heightmap: &HeightMap) -> Vec<[f32; 3]> {
    let w = heightmap.width();
    let h = heightmap.height();
    let s = heightmap.scale();

    let sample = |xi: usize, zi: usize, dx: i32, dz: i32| -> f32 {
        let nx = (xi as i32 + dx).clamp(0, w as i32 - 1) as usize;
        let nz = (zi as i32 + dz).clamp(0, h as i32 - 1) as usize;
        heightmap.get(nx, nz)
    };

    let mut normals = Vec::with_capacity(w * h);
    for zi in 0..h {
        for xi in 0..w {
            // Sobel X kernel: horizontal gradient (dh/dx direction)
            //  -1  0  1
            //  -2  0  2
            //  -1  0  1
            let gx = -sample(xi, zi, -1, -1) + sample(xi, zi, 1, -1)
                + -2.0 * sample(xi, zi, -1, 0) + 2.0 * sample(xi, zi, 1, 0)
                + -sample(xi, zi, -1, 1) + sample(xi, zi, 1, 1);

            // Sobel Z kernel: vertical gradient (dh/dz direction)
            //  -1 -2 -1
            //   0  0  0
            //   1  2  1
            let gz = -sample(xi, zi, -1, -1) - 2.0 * sample(xi, zi, 0, -1)
                - sample(xi, zi, 1, -1)
                + sample(xi, zi, -1, 1) + 2.0 * sample(xi, zi, 0, 1)
                + sample(xi, zi, 1, 1);

            let n = Vec3::new(-gx, 8.0 * s, -gz);
            let len = n.length();
            normals.push(if len > f32::EPSILON {
                (n / len).into()
            } else {
                [0.0, 1.0, 0.0]
            });
        }
    }
    normals
}
