use azterisk_render::prelude::*;

fn main() {
    println!("============================================================");
    println!("           AZTERISK GRAPHICS ENGINE - 2D SANDBOX            ");
    println!("============================================================");

    // 1. Define window in 2D render mode (mode = 0)
    let mut scene = Scene::define(
        1280,
        720,
        60,
        "Azterisk Render - 2D Grid Mode",
        0, // 2D Mode
        100,
        64,
    );

    // 2. Add colored 2D tiles and sprites on the grid
    scene
        .add(1, "0x0x0", "0,0,0")
        .color("#00FFCC")
        .scale(40.0);

    scene
        .add(1, "0x0x0", "-15,10,0")
        .color("#FF0055")
        .scale(25.0);

    scene
        .add(1, "0x0x0", "15,10,0")
        .color("#FFCC00")
        .scale(25.0);

    scene
        .add(1, "0x0x0", "0,-15,0")
        .color("#6600FF")
        .scale(30.0);

    // 3. Launch 2D loop
    scene.run();
}
