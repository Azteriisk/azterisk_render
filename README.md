# azterisk_render

A modular 3D and 2D graphics engine and library written in Rust, built natively on Vulkan via `wgpu`. Designed for grid-based worlds, procedural voxel simulations, and standalone desktop applications.

---

## Features

- **Spatial Coordinate System**: Deterministic two-tier grid architecture. Macro-chunks (`ChunkCoord3D`) define world-scale boundaries, while micro-cells (`SubCoord3D`) provide discrete integer block coordinates to prevent floating-point precision loss at scale.
- **Procedural Voxel Engine**:
  - `VoxelVolume<T>` generic 3D container supporting arbitrary data payloads.
  - Functional density field population (`fill(|x, y, z| ...)`).
  - `CulledBlockMesher`: Hidden-face culling generating single-mesh chunks with vertex ambient occlusion and anisotropy-aware diagonal triangulation.
  - 3D Digital Differential Analyzer (`raycast_voxel`) for discrete grid line-of-sight checks, block excavation, and placement.
- **Dynamic Lighting Pipeline**:
  - Single-pass uniform buffer supporting up to 16 concurrent dynamic lights.
  - Directional sunlight, quadratic point light attenuation, and spotlights with inner/outer cone falloff.
  - Light cookies / gobo projective texturing.
- **Batched 2D UI & Typography**:
  - Embedded `fontdue` rasterizer with default sans-serif font.
  - Atlas-backed glyph generation. UI boxes, borders, and text labels render within a single draw pass.
- **Input & Camera Controls**:
  - First-person, third-person orbital, and 2D orthographic camera modes.
  - Kinematic smoothing, configurable mouse inertia, and head bobbing cadence.
  - Continuous key state polling (`is_key_down`) and event-driven key/click handlers.
- **Spatial Audio**: Positional 3D sound listeners and 2D sound effect playback via `rodio`.
- **Asset Ingestion**: Native loaders for unit cubes, MagicaVoxel (`.vox`), Wavefront (`.obj`), and Autodesk FBX (`.fbx`).

---

## Example Usage

```rust
use azterisk_render::prelude::*;

fn main() {
    let mut scene = Scene::define(1920, 1080, 60, "Azterisk Engine Demo", 1, 100, 24);

    // Configure third-person camera
    scene.camera("Main", "0x0x0")
        .fov(75.0)
        .third_person(6.0, 2.0)
        .bind_default_controls(|ctrl| {
            ctrl.head_bob(true)
                .mouse_smoothing(0.80);
        });

    // Sun light
    scene.light("Sun", LightType::Directional, "0x0x0", "0,20,0")
        .direction(-0.35, -0.85, -0.40)
        .color("#FFF7ED")
        .intensity(12.0);

    // Procedural terrain volume
    let mut volume = VoxelVolume::new(24, 0u8);
    volume.fill(|x, y, z| {
        let height = ((x as f32 * 0.2).sin() * 4.0) as i32 - 2;
        if y > height { 0 } else { 1 }
    });

    let mesher = CulledBlockMesher::new(
        |b| *b > 0,
        |_b, _pos| Vec4::new(0.3, 0.7, 0.4, 1.0),
    );

    let terrain = scene.add(mesher.mesh(&volume), "0x0x0", "0,0,0");
    let terrain_id = terrain.id;

    // Interactive mining and building
    let mesher_clone = mesher.clone();
    scene.on_mouse_click(move |sc, button| {
        let cam = sc.active_cam();
        if let Some(hit) = raycast_voxel(&volume, cam.eye_position(), cam.forward(), 16.0, |b| *b > 0) {
            match button {
                MouseButton::Left => {
                    volume.set(hit.block_pos.x, hit.block_pos.y, hit.block_pos.z, 0);
                    sc.find_entity_mut(terrain_id).unwrap().update_mesh(mesher_clone.mesh(&volume));
                }
                MouseButton::Right => {
                    let p = hit.block_pos + hit.normal;
                    volume.set(p.x, p.y, p.z, 1);
                    sc.find_entity_mut(terrain_id).unwrap().update_mesh(mesher_clone.mesh(&volume));
                }
                _ => {}
            }
        }
    });

    scene.run();
}
```

---

## Building and Running

### Prerequisites

- Rust 1.80+
- Vulkan driver (`vulkan-tools`, `libvulkan-dev` or GPU vendor drivers on Linux)

### Run Examples

Procedural terrain sandbox with real-time editing:
```bash
cargo run --example procedural_world
```

3D model loading and gobo light cookies:
```bash
cargo run --example sandbox
```

2D orthographic sprite and UI pass:
```bash
cargo run --example sandbox_2d
```

### Run Tests

```bash
cargo test
```
