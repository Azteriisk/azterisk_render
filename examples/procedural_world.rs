use azterisk_render::prelude::*;
use glam::{IVec3, Vec4};

#[derive(Copy, Clone, PartialEq, Eq, Default)]
enum BlockType {
    #[default]
    Air,
    Grass,
    Dirt,
    Stone,
    GlowCrystal,
}

fn main() {
    println!("============================================================");
    println!("       AZTERISK GRAPHICS ENGINE - PROCEDURAL VOXEL WORLD    ");
    println!("============================================================");
    println!(" Controls:");
    println!("   W / S       : Move forward / backward along voxel terrain");
    println!("   A / D       : Strafe left / right");
    println!("   Space/Shift : Step up / down");
    println!("   Mouse       : Look around with smooth camera inertia");
    println!("   V / F5      : Toggle 1st Person / 3rd Person view");
    println!("   ESC         : Exit");
    println!("============================================================");

    let subdivisions = 24; // 49x49x49 grid = 117,649 possible voxel locations
    let mut scene = Scene::define(
        1920,
        1080,
        60,
        "Azterisk Render - Procedural Voxel World",
        1,
        100,
        subdivisions,
    );

    // 1. Camera setup with realistic inertia and 3rd person orbit
    scene
        .camera("Explorer", "0x0x0")
        .fov(80.0)
        .third_person(6.0, 2.0)
        .attach_model(1, "#38BDF8")
        .player_model_scale(0.9)
        .bind_default_controls(|ctrl| {
            ctrl.grid_stepper()
                .step_size(1)
                .transition_time_ms(150)
                .easing(Ease::InOutQuad);

            ctrl.head_bob(true)
                .head_bob_intensity(0.16)
                .head_bob_frequency(12.0)
                .mouse_smoothing(0.80)
                .look_sensitivity(0.15);
        });

    // 2. Multi-Light System: Sunlight + Underground Glow Crystals + Surface Campfire
    scene
        .light("Sun", LightType::Directional, "0x0x0", "0,20,0")
        .direction(-0.35, -0.85, -0.40)
        .color("#FFF7ED")
        .intensity(12.0);

    scene
        .light("Campfire", LightType::Point, "0x0x0", "0,0,0")
        .color("#F59E0B")
        .intensity(22.0)
        .radius(24.0);

    scene
        .light("CaveCrystal", LightType::Point, "0x0x0", "4,-8,4")
        .color("#06B6D4")
        .intensity(18.0)
        .radius(20.0);

    // 3. Populate Voxel Volume procedurally with rolling hills and hollow caves
    println!("[Engine] Generating procedural 3D voxel density field...");
    let mut volume = VoxelVolume::new(subdivisions, BlockType::Air);

    volume.fill(|x, y, z| {
        // Multi-frequency sinusoidal terrain heightmap simulating rolling hills
        let fx = x as f32 * 0.18;
        let fz = z as f32 * 0.18;
        let height = (fx.sin() * 4.5 + fz.cos() * 4.0 + (fx * 0.5 + fz * 0.5).sin() * 2.0) as i32 - 4;

        // 3D cavern noise: carve hollow tunnel voids through subterranean rock
        let cave_density = (x as f32 * 0.35).sin() * (y as f32 * 0.35).cos() * (z as f32 * 0.35).sin();
        let is_cave = y < height - 2 && cave_density > 0.45;

        if is_cave {
            // Rare glowing crystal nodes inside deep cave pockets
            if cave_density > 0.65 && y < -10 {
                BlockType::GlowCrystal
            } else {
                BlockType::Air
            }
        } else if y > height {
            BlockType::Air
        } else if y == height {
            BlockType::Grass
        } else if y >= height - 2 {
            BlockType::Dirt
        } else {
            BlockType::Stone
        }
    });

    // 4. Mesh the entire chunk with CulledBlockMesher -> 1 Draw Call!
    println!("[Engine] Meshing chunk with face culling...");
    let mesher = CulledBlockMesher::new(
        |block| *block != BlockType::Air,
        |block, pos: IVec3| match block {
            BlockType::Grass => {
                // Subtle color variation across terrain
                let tint = ((pos.x + pos.z) % 3) as f32 * 0.04;
                Vec4::new(0.13 + tint, 0.77 + tint, 0.36, 1.0)
            }
            BlockType::Dirt => Vec4::new(0.47, 0.21, 0.06, 1.0),
            BlockType::Stone => {
                let gray = 0.35 + ((pos.x.abs() * 7 + pos.z.abs() * 13) % 5) as f32 * 0.03;
                Vec4::new(gray, gray, gray * 1.05, 1.0)
            }
            BlockType::GlowCrystal => Vec4::new(0.02, 0.85, 0.95, 1.0),
            BlockType::Air => Vec4::ZERO,
        },
    );

    let world_mesh = mesher.mesh(&volume);
    println!(
        "[Engine] Chunk mesh generated: {} vertices, {} triangles (Rendered in 1 draw call)",
        world_mesh.vertices.len(),
        world_mesh.indices.len() / 3
    );

    let terrain = scene.add(world_mesh, "0x0x0", "0,0,0");
    let terrain_id = terrain.id;

    // 5. Interactive Real-Time Dig & Build Gameplay via 3D DDA Raycasting
    let mesher_clone = mesher.clone();
    scene.on_mouse_click(move |sc, button| {
        let cam = sc.active_cam();
        let ray_origin = cam.eye_position();
        let ray_dir = cam.forward();

        if let Some(hit) = raycast_voxel(&volume, ray_origin, ray_dir, 20.0, |b| *b != BlockType::Air) {
            match button {
                MouseButton::Left => {
                    // Mine targeted voxel block -> Air
                    volume.set(hit.block_pos.x, hit.block_pos.y, hit.block_pos.z, BlockType::Air);
                    let new_mesh = mesher_clone.mesh(&volume);
                    if let Some(ent) = sc.find_entity_mut(terrain_id) {
                        ent.update_mesh(new_mesh);
                    }
                    sc.audio.play_sfx("assets/audio/pop.wav", 1.0);
                    println!(
                        "[Engine] Mined block at ({}, {}, {})",
                        hit.block_pos.x, hit.block_pos.y, hit.block_pos.z
                    );
                }
                MouseButton::Right => {
                    // Build new Stone block on adjacent face normal
                    let place_pos = hit.block_pos + hit.normal;
                    if volume.get(place_pos.x, place_pos.y, place_pos.z).is_some() {
                        volume.set(place_pos.x, place_pos.y, place_pos.z, BlockType::Stone);
                        let new_mesh = mesher_clone.mesh(&volume);
                        if let Some(ent) = sc.find_entity_mut(terrain_id) {
                            ent.update_mesh(new_mesh);
                        }
                        sc.audio.play_sfx("assets/audio/click.wav", 1.0);
                        println!(
                            "[Engine] Placed Stone at ({}, {}, {})",
                            place_pos.x, place_pos.y, place_pos.z
                        );
                    }
                }
                _ => {}
            }
        }
    });

    // 6. 3D Spatial Audio beacon humming inside the valley
    scene
        .sound("assets/audio/beacon_hum.wav", "0x0x0", "0,-4,0")
        .radius(40.0)
        .intensity(0.85)
        .looping(true);

    // 7. UI Overlay with sharp typography
    scene.add_ui(
        UIElement::new("title_badge")
            .anchor(Anchor::TopLeft)
            .offset(24.0, 24.0)
            .size(420.0, 44.0)
            .color("#0F172A")
            .border(1.5, "#22C55E")
            .radius(6.0)
            .text("Azterisk Render | Voxel World Sandbox")
            .text_size(15.0),
    );

    scene.add_ui(
        UIElement::new("hud_stats")
            .anchor(Anchor::BottomCenter)
            .offset(0.0, -28.0)
            .size(620.0, 42.0)
            .color("#1E293B")
            .hover_color("#334155")
            .border(1.5, "#38BDF8")
            .radius(6.0)
            .text("[L-Click] Mine | [R-Click] Place | [W/A/S/D] Move | [V] 3rd Person")
            .text_size(14.0),
    );

    // 8. Launch the engine
    scene.run();
}
