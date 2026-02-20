# bevy_symbios_ground

Bevy integration for the [`symbios-ground`](https://crates.io/crates/symbios-ground) algorithmic terrain engine.

Translates `HeightMap` and `WeightMap` data from `symbios-ground` into Bevy-compatible meshes, physics colliders, and GPU splat textures.

---

## Features

- **Mesh generation** — Convert a `HeightMap` to a Bevy `Mesh` with `TriangleList` topology, smooth per-vertex normals, and tiling UV coordinates.
- **Normal methods** — Choose between area-weighted triangle normals (accurate for jagged terrain) or a Sobel filter (smooth, faster for continuous procedural terrain).
- **Splat textures** — Convert a `WeightMap` to an RGBA8 GPU texture for use with terrain shaders.
- **Texture sync** — Bevy system to re-upload the splat texture on the next frame whenever terrain data changes.
- **Physics colliders** *(optional, `physics` feature)* — Generate an Avian3D `Collider::heightfield` from a `HeightMap`.

---

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
bevy = "0.18"
bevy_symbios_ground = "0.1"
symbios-ground = "0.1"
```

To enable Avian3D physics collider generation:

```toml
[dependencies]
bevy_symbios_ground = { version = "0.1", features = ["physics"] }
avian3d = "0.5"
```

---

## Quick Start

### Terrain mesh

```rust
use bevy::prelude::*;
use bevy_symbios_ground::HeightMapMeshBuilder;
use symbios_ground::{HeightMap, generators::FbmNoise, TerrainGenerator};

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut heightmap = HeightMap::new(128, 128, 1.0);
    FbmNoise::default().generate(&mut heightmap);
    heightmap.normalize();

    let mesh = HeightMapMeshBuilder::new()
        .with_uv_tile_size(4.0)
        .build(&heightmap);

    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
    ));
}
```

### Splat texture

```rust
use bevy::prelude::*;
use bevy_symbios_ground::{GroundMaterialSettings, SplatTexture, splat_to_image, sync_splat_texture};
use symbios_ground::{HeightMap, SplatMapper};

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let heightmap = HeightMap::new(128, 128, 1.0);
    let weight_map = SplatMapper::default().generate(&heightmap);
    let image = splat_to_image(&weight_map);

    commands.insert_resource(SplatTexture { handle: images.add(image) });
    commands.insert_resource(GroundMaterialSettings::new(weight_map));
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, sync_splat_texture)
        .run();
}
```

### Physics collider *(requires `physics` feature)*

```rust
use bevy_symbios_ground::build_heightfield_collider;
use symbios_ground::HeightMap;

let heightmap = HeightMap::new(64, 64, 1.0);
let collider = build_heightfield_collider(&heightmap);
// commands.spawn((collider, Transform::from_xyz(-32.0, 0.0, -32.0), ...));
```

The heightfield collider is centered at the origin of its local space. Offset the entity's `Transform` by `(-world_width/2, 0, -world_depth/2)` to align it with a mesh generated from the same `HeightMap`.

---

## API Overview

### Mesh generation

| Item | Description |
|------|-------------|
| [`HeightMapMeshBuilder`] | Builder that converts a `HeightMap` to a Bevy `Mesh`. |
| [`NormalMethod`] | Selects the normal-computation algorithm: `AreaWeighted` or `Sobel`. |

**`HeightMapMeshBuilder` methods:**

| Method | Default | Description |
|--------|---------|-------------|
| `with_uv_tile_size(f32)` | `1.0` | World-space size of one UV tile. |
| `with_normal_method(NormalMethod)` | `AreaWeighted` | Normal computation algorithm. |
| `build(&HeightMap) -> Mesh` | — | Builds the mesh, consuming the builder. |

#### Normal methods

- **`AreaWeighted`** — Accumulates unnormalized cross-products (proportional to triangle area) at each vertex, then normalizes. Most accurate for jagged or eroded terrain.
- **`Sobel`** — Applies 3×3 Sobel kernels to the heightmap grid to derive normals analytically. Faster and produces smoother results; best for continuous procedural terrain.

### Splat textures

| Item | Description |
|------|-------------|
| `splat_to_image(&WeightMap) -> Image` | Converts a `WeightMap` to an RGBA8Unorm Bevy `Image`. |
| `GroundMaterialSettings` | Resource holding the current `WeightMap` and dirty flag. |
| `SplatTexture` | Resource holding the GPU-side `Handle<Image>`. |
| `sync_splat_texture` | Bevy system that re-uploads the texture when the resource is marked dirty. |

To trigger a re-upload, call `settings.mark_dirty()` after modifying `settings.weight_map`.

### Physics colliders *(feature: `physics`)*

| Item | Description |
|------|-------------|
| `build_heightfield_collider(&HeightMap) -> Collider` | Builds an Avian3D `Collider::heightfield`. |

---

## Feature Flags

| Flag | Default | Description |
|------|---------|-------------|
| `physics` | off | Enables Avian3D `Collider::heightfield` generation via `build_heightfield_collider`. |

---

## Compatibility

| `bevy_symbios_ground` | Bevy | symbios-ground | avian3d (`physics`) |
|----------------------|------|----------------|---------------------|
| `0.1` | `0.18` | `0.1` | `0.5` |

---

## License

MIT — see [LICENSE](LICENSE).
