use azterisk_render::prelude::*;

fn main() {
    println!("============================================================");
    println!("           AZTERISK GRAPHICS ENGINE - 3D SANDBOX            ");
    println!("============================================================");
    println!(" Controls:");
    println!("   W / S       : Step forward / backward along grid");
    println!("   A / D       : Step left / right along grid");
    println!("   Space/Shift : Step up / down");
    println!("   Mouse       : Look around (with smooth realistic inertia)");
    println!("   V / F5      : Toggle between 1st Person & 3rd Person view");
    println!("   ESC         : Exit");
    println!("============================================================");

    // 1. Define window and spatial world space (1920x1080 @ 60fps, 3D, span 100, 64 subdivisions)
    let mut scene = Scene::define(
        1920,
        1080,
        60,
        "Azterisk Render - Camera Dynamics & 3rd Person Sandbox",
        1,
        100,
        64,
    );

    // 2. Define camera with:
    //    - Custom FOV (75.0 degrees)
    //    - Head bobbing enabled with bounce cadence
    //    - Realistic mouse smoothing / inertia delay (0.82)
    //    - 3rd person orbit settings & player model attachment
    scene
        .camera("Player", "0x0x0")
        .fov(75.0)
        .third_person(5.5, 1.8) // Distance = 5.5 units, Height = 1.8 units
        .attach_model(1, "#3388FF") // Cyan-blue player avatar block!
        .player_model_scale(1.0)
        .bind_default_controls(|ctrl| {
            ctrl.grid_stepper()
                .step_size(1)
                .transition_time_ms(160)
                .easing(Ease::InOutQuad);

            // Realistic camera dynamics:
            ctrl.head_bob(true)
                .head_bob_intensity(0.18)
                .head_bob_frequency(12.0)
                .mouse_smoothing(0.82) // Filmic inertia / delay when moving mouse
                .look_sensitivity(0.16);
        });

    // 3. Set up spotlight with a film Gobo / Cookie mask
    scene
        .light("KeyLight", LightType::Spot, "0x0x1", "0,15,-10")
        .direction(0.0, -0.7, 0.7)
        .color("#FFF4E0")
        .intensity(28.0)
        .radius(80.0)
        .cone(30.0, 50.0)
        .gobo("assets/textures/window_blinds.png")
        .expect("Failed to attach gobo texture");

    // 4. Add voxel cubes anchored at chunk 0x0x1 directly ahead of camera
    // Emerald green center block
    scene
        .add(1, "0x0x1", "0,0,0")
        .color("#00FF88");

    // Surrounding colored blocks
    scene
        .add(1, "0x0x1", "-4,0,0")
        .color("#FF3366");

    scene
        .add(1, "0x0x1", "4,0,0")
        .color("#3399FF");

    scene
        .add(1, "0x0x1", "0,4,0")
        .color("#FFAA00");

    scene
        .add(1, "0x0x1", "0,-4,0")
        .color("#AA33FF");

    // Add a checkered floor of voxel blocks to see the gobo shadows and head-bob motion clearly
    for x in -7..=7 {
        for z in -7..=7 {
            if (x + z) % 2 == 0 {
                scene
                    .add(1, "0x0x1", (x * 2, -6, z * 2))
                    .color("#384252");
            }
        }
    }

    // 5. Add 3D Positional Audio (Humming beacon anchored at center block)
    scene
        .sound("assets/audio/beacon_hum.wav", "0x0x1", "0,0,0")
        .radius(35.0)
        .intensity(0.8)
        .looping(true);

    // 6. Add Retained UI Overlay Elements
    // Top-Left Status Badge
    scene.add_ui(
        UIElement::new("status_badge")
            .anchor(Anchor::TopLeft)
            .offset(24.0, 24.0)
            .size(320.0, 42.0)
            .color("#111827")
            .border(1.0, "#00FF88")
            .radius(6.0)
            .text("Azterisk Render | [V / F5] 3rd Person"),
    );

    // Bottom-Center Interactive Button
    scene.add_ui(
        UIElement::new("hud_button")
            .anchor(Anchor::BottomCenter)
            .offset(0.0, -32.0)
            .size(240.0, 48.0)
            .color("#1E293B")
            .hover_color("#334155")
            .border(2.0, "#38BDF8")
            .radius(8.0)
            .text("Action HUD Button")
            .on_click(|_elem| {
                println!("[UI Event] Action button was clicked!");
            }),
    );

    // 7. Run the engine!
    scene.run();
}
