use azterisk_render::prelude::*;

#[test]
fn test_hex_color_parsing() {
    let red: glam::Vec3 = "#FF0000".into_color();
    assert!((red.x - 1.0).abs() < 0.01);
    assert!(red.y < 0.01);
    assert!(red.z < 0.01);

    let green: glam::Vec3 = "#00FF88".into_color();
    assert!(green.x < 0.01);
    assert!((green.y - 1.0).abs() < 0.01);
    assert!((green.z - 0.533).abs() < 0.02);
}

#[test]
fn test_diagnostic_error_messages() {
    let out_of_bounds = ChunkCoord3D::new(150, 0, 0);
    let err = out_of_bounds.validate(100).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Chunk coordinate (150, 0, 0) is out of bounds"));
    assert!(msg.contains("Allowed range: [-100 ..= 100]"));
    assert!(msg.contains("Hint: Increase world_span in `Scene::define"));

    let sub_out_of_bounds = SubCoord3D::new(0, 99, 0);
    let sub_err = sub_out_of_bounds.validate(64).unwrap_err();
    let sub_msg = sub_err.to_string();
    assert!(sub_msg.contains("Sub-cell / block coordinate (0, 99, 0) is out of bounds"));
    assert!(sub_msg.contains("Allowed range: [-64 ..= 64]"));
}

#[test]
fn test_camera_stepping_across_chunk_boundaries() {
    let mut cam = Camera::new("Player", ChunkCoord3D::new(0, 0, 0), SubCoord3D::new(64, 0, 0), 64);
    assert_eq!(cam.chunk, ChunkCoord3D::new(0, 0, 0));
    assert_eq!(cam.sub, SubCoord3D::new(64, 0, 0));

    cam.step(1, 0, 0, 100, 64).unwrap();
    assert_eq!(cam.chunk, ChunkCoord3D::new(1, 0, 0));
    assert_eq!(cam.sub, SubCoord3D::new(-64, 0, 0));
}

#[test]
fn test_camera_smooth_interpolation() {
    let mut cam = Camera::new("Player", ChunkCoord3D::new(0, 0, 0), SubCoord3D::new(0, 0, 0), 64);
    cam.stepper.transition_time_ms = 100;
    cam.step(0, 0, 1, 100, 64).unwrap();

    assert!(cam.is_transitioning);
    assert_eq!(cam.transition_progress, 0.0);

    cam.update(0.05);
    assert!(cam.is_transitioning);
    assert!(cam.transition_progress > 0.4 && cam.transition_progress < 0.6);

    cam.update(0.06);
    assert!(!cam.is_transitioning);
    assert_eq!(cam.transition_progress, 1.0);
    assert_eq!(cam.current_world_pos, cam.target_world_pos);
}

#[test]
fn test_camera_fov_and_third_person() {
    let mut cam = Camera::new("Player", ChunkCoord3D::new(0, 0, 0), SubCoord3D::new(0, 0, 0), 64);
    cam.fov(85.0);
    assert_eq!(cam.fov_deg, 85.0);

    // Test 3rd person mode setup
    cam.third_person(8.0, 2.5);
    assert_eq!(cam.mode, CameraMode::ThirdPerson);
    assert_eq!(cam.third_person_distance, 8.0);
    assert_eq!(cam.third_person_height, 2.5);

    // Eye position should be behind the player
    let eye = cam.eye_position();
    assert!(eye.z > cam.current_world_pos.z || eye.y > cam.current_world_pos.y);

    // Test toggle perspective
    cam.toggle_perspective();
    assert_eq!(cam.mode, CameraMode::FirstPerson);
}

#[test]
fn test_camera_head_bobbing() {
    let mut cam = Camera::new("Player", ChunkCoord3D::new(0, 0, 0), SubCoord3D::new(0, 0, 0), 64);
    cam.head_bob_enabled = true;
    cam.head_bob_intensity = 0.2;
    cam.head_bob_frequency = 10.0;

    // Trigger grid step
    cam.step(0, 0, 1, 100, 64).unwrap();
    assert!(cam.is_transitioning);

    // Simulate movement frame
    cam.update(0.05);
    // Head bob offset should be non-zero during movement
    assert!(cam.current_bob_offset.y > 0.0 || cam.current_bob_offset.x != 0.0);
}

#[test]
fn test_camera_mouse_smoothing_inertia() {
    let mut cam = Camera::new("Player", ChunkCoord3D::new(0, 0, 0), SubCoord3D::new(0, 0, 0), 64);
    cam.mouse_smoothing = 0.85;

    // Simulate raw mouse motion
    cam.rotate(100.0, 0.0);
    // Immediately after mouse motion, target yaw has changed but smoothed yaw is lagging behind
    assert!(cam.target_yaw_deg > -90.0);
    assert_eq!(cam.yaw_deg, -90.0);

    // Advance 1 frame (16.6ms)
    cam.update(0.0166);
    // Yaw smoothly moves towards target with delay
    assert!(cam.yaw_deg > -90.0);
    assert!(cam.yaw_deg < cam.target_yaw_deg);
}

#[test]
fn test_scene_definition_and_defaults() {
    let scene = Scene::define(1920, 1080, 60, "Test Scene", 1, 100, 64);
    assert_eq!(scene.width, 1920);
    assert_eq!(scene.height, 1080);
    assert_eq!(scene.fps, 60);
    assert_eq!(scene.mode, RenderMode::ThreeD);
    assert_eq!(scene.world_span, 100);
    assert_eq!(scene.subdivisions, 64);
}

#[test]
fn test_ui_element_bounds_and_hit_testing() {
    let screen_w = 1920.0;
    let screen_h = 1080.0;

    // Top-Left anchored button (100x50 at offset 10, 10)
    let btn = UIElement::new("test_button")
        .anchor(Anchor::TopLeft)
        .offset(10.0, 10.0)
        .size(100.0, 50.0)
        .color("#222222")
        .hover_color("#444444")
        .sound("click.wav")
        .radius(6.0);

    assert_eq!(btn.radius, 6.0);
    assert_eq!(btn.sound_path, Some("click.wav".to_string()));

    // Bounds: [10, 10, 110, 60]
    let bounds = btn.compute_bounds(screen_w, screen_h);
    assert_eq!(bounds, [10.0, 10.0, 110.0, 60.0]);

    assert!(btn.hit_test(50.0, 30.0, screen_w, screen_h));
    assert!(!btn.hit_test(5.0, 30.0, screen_w, screen_h));
    assert!(!btn.hit_test(150.0, 30.0, screen_w, screen_h));

    // Center anchored UI element (200x100 at center of screen)
    let center_box = UIElement::new("center_dialog")
        .anchor(Anchor::Center)
        .offset(0.0, 0.0)
        .size(200.0, 100.0);

    let center_bounds = center_box.compute_bounds(screen_w, screen_h);
    // Center of 1920x1080 is (960, 540) -> bounds should be [860, 490, 1060, 590]
    assert_eq!(center_bounds, [860.0, 490.0, 1060.0, 590.0]);
    assert!(center_box.hit_test(960.0, 540.0, screen_w, screen_h));
}

#[test]
fn test_spatial_sound_configuration() {
    let mut sound = SpatialSound::new("ambient.ogg", ChunkCoord3D::new(0, 0, 0), SubCoord3D::new(0, 0, 0), 64);
    sound.radius(45.0).intensity(0.8).looping(true);

    assert_eq!(sound.radius, 45.0);
    assert_eq!(sound.intensity, 0.8);
    assert!(sound.is_looping);
}

#[test]
fn test_mesh_into_asset_direct_ingestion() {
    let mut scene = Scene::define(1280, 720, 60, "Mesh Test", 1, 10, 64);
    let custom_mesh = Mesh::unit_cube(glam::Vec4::ONE);

    // Pass custom Mesh directly into scene.add
    let entity = scene.add(custom_mesh, "0x0x0", "0,0,0");
    assert_eq!(entity.id, 1);
    assert_eq!(entity.mesh.vertices.len(), 24);
    assert_eq!(entity.mesh.indices.len(), 36);
}

#[test]
fn test_font_atlas_rasterization_and_measurement() {
    let atlas = FontAtlas::new(azterisk_render::text::DEFAULT_FONT_BYTES, 48.0)
        .expect("Default font failed to load");
    assert!(atlas.glyphs.len() >= 90);

    // Verify ASCII glyph 'A'
    let glyph_a = atlas.glyphs.get(&'A').expect("Glyph 'A' missing");
    assert!(glyph_a.width > 0.0);
    assert!(glyph_a.height > 0.0);
    assert!(glyph_a.advance_width > 0.0);
    assert!(glyph_a.uv_max.x > glyph_a.uv_min.x);
    assert!(glyph_a.uv_max.y > glyph_a.uv_min.y);

    // Measure string
    let measured = atlas.measure_text("Azterisk Render", 24.0);
    assert!(measured.x > 50.0);
    assert!(measured.y > 10.0);
}

#[test]
fn test_ui_element_mesh_with_text() {
    let atlas = FontAtlas::new(azterisk_render::text::DEFAULT_FONT_BYTES, 48.0)
        .expect("Font failed to load");
    let btn = UIElement::new("test_btn")
        .size(200.0, 50.0)
        .border(2.0, "#00FF88")
        .text("Play Game")
        .text_size(18.0);

    let mesh = btn.generate_mesh_with_font(1920.0, 1080.0, Some(&atlas));
    // Should have: border quad + body quad + 8 letter quads = 40 vertices
    assert!(mesh.vertices.len() >= 30);
    assert!(mesh.indices.len() >= 45);

    // Check that some vertices have text UVs (>= 0.0) and some have solid UVs (< 0.0)
    let has_solid_quad = mesh.vertices.iter().any(|v| v.uv[0] < 0.0);
    let has_text_quad = mesh.vertices.iter().any(|v| v.uv[0] >= 0.0);
    assert!(has_solid_quad);
    assert!(has_text_quad);
}

#[test]
fn test_voxel_volume_fill_and_carving() {
    let mut vol: VoxelVolume<u8> = VoxelVolume::new(16, 0);
    assert_eq!(vol.diameter, 33);

    // Set and get
    vol.set(0, 0, 0, 1);
    assert_eq!(vol.get(0, 0, 0), Some(&1));
    assert_eq!(vol.get(1, 0, 0), Some(&0));

    // Carve sphere
    vol.set(2, 2, 2, 5);
    vol.carve_sphere(glam::IVec3::new(2, 2, 2), 1.5, 0);
    assert_eq!(vol.get(2, 2, 2), Some(&0));
}

#[test]
fn test_culled_block_mesher_internal_face_culling() {
    let mut vol: VoxelVolume<u8> = VoxelVolume::new(8, 0);
    let mesher = CulledBlockMesher::new(
        |val| *val > 0,
        |_val, _pos| glam::Vec4::ONE,
    );

    // Case A: 1 single block has 6 exposed faces = 24 vertices, 36 indices
    vol.set(0, 0, 0, 1);
    let single_mesh = mesher.mesh(&vol);
    assert_eq!(single_mesh.vertices.len(), 24);
    assert_eq!(single_mesh.indices.len(), 36);

    // Case B: 2 adjacent blocks share 1 internal face.
    // Naive = 12 faces (48 vertices). Culled = 10 faces (40 vertices)!
    vol.set(1, 0, 0, 1);
    let double_mesh = mesher.mesh(&vol);
    assert_eq!(double_mesh.vertices.len(), 40);
    assert_eq!(double_mesh.indices.len(), 60);

    // Case C: 2x2x2 cube of 8 blocks.
    // Naive = 8 * 6 = 48 faces. Culled = 24 outer faces (96 vertices)!
    let mut cube_vol: VoxelVolume<u8> = VoxelVolume::new(8, 0);
    for x in 0..2 {
        for y in 0..2 {
            for z in 0..2 {
                cube_vol.set(x, y, z, 1);
            }
        }
    }
    let cube_mesh = mesher.mesh(&cube_vol);
    assert_eq!(cube_mesh.vertices.len(), 96);
    assert_eq!(cube_mesh.indices.len(), 144);
}

#[test]
fn test_raycast_voxel_dda() {
    let mut vol: VoxelVolume<u8> = VoxelVolume::new(16, 0);
    vol.set(0, 0, 0, 1); // Solid target block at (0, 0, 0)

    // Raycast from (0.0, 0.0, 5.0) pointing towards -Z (0, 0, -1)
    let hit = raycast_voxel(
        &vol,
        glam::Vec3::new(0.0, 0.0, 5.0),
        glam::Vec3::new(0.0, 0.0, -1.0),
        10.0,
        |val| *val > 0,
    );

    assert!(hit.is_some());
    let hit = hit.unwrap();
    assert_eq!(hit.block_pos, glam::IVec3::new(0, 0, 0));
    assert_eq!(hit.normal, glam::IVec3::new(0, 0, 1)); // Front (+Z) face

    // Test miss
    let miss = raycast_voxel(
        &vol,
        glam::Vec3::new(0.0, 10.0, 5.0),
        glam::Vec3::new(0.0, 0.0, -1.0),
        10.0,
        |val| *val > 0,
    );
    assert!(miss.is_none());
}

#[test]
fn test_entity_dynamic_mesh_update() {
    let mut ent = Entity::new(
        1,
        ChunkCoord3D::new(0, 0, 0),
        SubCoord3D::new(0, 0, 0),
        Mesh::default(),
        16,
    );
    assert!(!ent.is_mesh_dirty);

    let new_mesh = Mesh::unit_cube(glam::Vec4::ONE);
    ent.update_mesh(new_mesh);
    assert!(ent.is_mesh_dirty);
    assert_eq!(ent.mesh.vertices.len(), 24);
}

#[test]
fn test_multi_light_uniform_packing() {
    assert_eq!(MAX_LIGHTS, 16);
    assert_eq!(std::mem::size_of::<LightsUniform>(), 2064);

    let mut lights_uniform = LightsUniform::default();
    assert_eq!(lights_uniform.count, 0);

    let l1 = Light::new("Sun", LightType::Directional, ChunkCoord3D::new(0, 0, 0), SubCoord3D::new(0, 10, 0), 16);
    let l2 = Light::new("Torch", LightType::Point, ChunkCoord3D::new(0, 0, 0), SubCoord3D::new(2, 0, 2), 16);
    let l3 = Light::new("Spot", LightType::Spot, ChunkCoord3D::new(0, 0, 0), SubCoord3D::new(0, 5, 0), 16);

    let scene_lights = vec![l1, l2, l3];
    lights_uniform.count = scene_lights.len() as u32;
    for (i, l) in scene_lights.iter().enumerate() {
        lights_uniform.lights[i] = l.to_uniform();
    }

    assert_eq!(lights_uniform.count, 3);
    // l1 is Directional (light_type = 1.0)
    assert_eq!(lights_uniform.lights[0].direction[3], 1.0);
    // l2 is Point (light_type = 2.0)
    assert_eq!(lights_uniform.lights[1].direction[3], 2.0);
    // l3 is Spot (light_type = 3.0)
    assert_eq!(lights_uniform.lights[2].direction[3], 3.0);
}

#[test]
fn test_scene_mouse_click_and_interactive_voxel_mining() {
    let subdivisions = 8;
    let mut scene = Scene::define(800, 600, 60, "Voxel Test", 1, 10, subdivisions);
    scene.camera("Cam", "0x0x0");

    let mut volume: VoxelVolume<u8> = VoxelVolume::new(subdivisions, 0);
    // Place solid block at (0, 0, -2)
    volume.set(0, 0, -2, 1);

    let mesher = CulledBlockMesher::new(
        |b| *b > 0,
        |_b, _pos| glam::Vec4::new(0.5, 0.5, 0.5, 1.0),
    );

    let initial_mesh = mesher.mesh(&volume);
    assert_eq!(initial_mesh.vertices.len(), 24); // 1 single cube = 24 vertices

    let ent = scene.add(initial_mesh, "0x0x0", "0,0,0");
    let ent_id = ent.id;

    // Test Left-Click Mining Simulation
    let cam = scene.active_cam();
    let origin = cam.eye_position();
    let dir = glam::Vec3::new(0.0, -0.25, -1.0).normalize();

    let hit = raycast_voxel(&volume, origin, dir, 10.0, |b| *b > 0);
    assert!(hit.is_some());
    let hit = hit.unwrap();
    assert_eq!(hit.block_pos, glam::IVec3::new(0, 0, -2));

    // Mine block
    volume.set(hit.block_pos.x, hit.block_pos.y, hit.block_pos.z, 0);
    let mined_mesh = mesher.mesh(&volume);
    assert_eq!(mined_mesh.vertices.len(), 0); // All air -> 0 vertices

    let ent = scene.find_entity_mut(ent_id).unwrap();
    ent.update_mesh(mined_mesh);
    assert!(ent.is_mesh_dirty);

    // Test Right-Click Building Simulation
    let build_pos = hit.block_pos + hit.normal;
    volume.set(build_pos.x, build_pos.y, build_pos.z, 2);
    let placed_mesh = mesher.mesh(&volume);
    assert_eq!(placed_mesh.vertices.len(), 24); // Newly placed block
    ent.update_mesh(placed_mesh);
    assert!(ent.is_mesh_dirty);
}

#[test]
fn test_cursor_capture_and_visibility_settings() {
    let mut scene = Scene::define(1280, 720, 60, "Cursor Test", 1, 100, 24);

    // Initial default state
    assert_eq!(scene.cursor_mode, CursorMode::Normal);
    assert!(scene.cursor_visible);
    assert!(matches!(scene.custom_cursor, CustomCursor::None));

    // Capture mouse
    scene.capture_mouse(true);
    assert_eq!(scene.cursor_mode, CursorMode::Captured);
    assert!(scene.cursor_state_dirty);

    // Release mouse
    scene.capture_mouse(false);
    assert_eq!(scene.cursor_mode, CursorMode::Normal);

    // Hide cursor
    scene.hide_cursor();
    assert!(!scene.cursor_visible);

    // Show cursor
    scene.show_cursor();
    assert!(scene.cursor_visible);

    // Replace cursor with custom crosshair
    scene.replace_cursor(CustomCursor::crosshair());
    assert!(!scene.cursor_visible); // Replaced cursor hides OS cursor
    assert!(matches!(scene.custom_cursor, CustomCursor::Crosshair { .. }));

    // Restore cursor
    scene.restore_cursor();
    assert!(scene.cursor_visible);
    assert!(matches!(scene.custom_cursor, CustomCursor::None));
}

#[test]
fn test_custom_cursor_mesh_generation() {
    let crosshair = CustomCursor::crosshair();
    let mesh = crosshair.generate_mesh(glam::Vec2::new(100.0, 50.0));
    assert!(!mesh.vertices.is_empty());
    assert!(!mesh.indices.is_empty());
    // All vertices must have negative UV to bypass font texture sampling in 2D shader
    assert!(mesh.vertices.iter().all(|v| v.uv[0] < 0.0 && v.uv[1] < 0.0));

    let dot = CustomCursor::dot();
    let dot_mesh = dot.generate_mesh(glam::Vec2::ZERO);
    assert!(!dot_mesh.vertices.is_empty());

    let pointer = CustomCursor::pointer();
    let pointer_mesh = pointer.generate_mesh(glam::Vec2::new(-20.0, 40.0));
    assert!(!pointer_mesh.vertices.is_empty());

    let ring = CustomCursor::ring();
    let ring_mesh = ring.generate_mesh(glam::Vec2::ZERO);
    assert!(!ring_mesh.vertices.is_empty());
}
