use thiserror::Error;

/// Informative, developer-friendly errors for the Azterisk graphics engine.
/// Designed so developers can understand what went wrong and how to fix it
/// directly from runtime and compiler messages without needing documentation.
#[derive(Error, Debug)]
pub enum AzteriskError {
    #[error("Chunk coordinate ({x}, {y}, {z}) is out of bounds!\n  Allowed range: [-{span} ..= {span}] in all axes.\n  Hint: Increase world_span in `Scene::define(..., world_span, ...)` or adjust your chunk coordinate.")]
    ChunkOutOfBounds {
        x: i32,
        y: i32,
        z: i32,
        span: i32,
    },

    #[error("Sub-cell / block coordinate ({x}, {y}, {z}) is out of bounds!\n  Allowed range: [-{subdivisions} ..= {subdivisions}] in all axes.\n  Hint: Keep sub-coordinates within [-{subdivisions}..={subdivisions}] or increase subdivisions in `Scene::define(..., subdivisions)`.")]
    SubCoordOutOfBounds {
        x: i32,
        y: i32,
        z: i32,
        subdivisions: i32,
    },

    #[error("Invalid coordinate string '{raw}'.\n  Expected 3D chunk format: \"X x Y x Z\" (e.g. \"0x0x1\", \"-2x5x10\").\n  Expected 2D chunk format: \"X x Y\" (e.g. \"0x1\").\n  Expected sub-cell format: \"X, Y, Z\" (e.g. \"0,0,0\") or \"X, Y\" (e.g. \"0,0\").")]
    InvalidCoordinateString {
        raw: String,
    },

    #[error("Failed to load asset '{path}': {details}\n  Hint: Ensure the file exists and is in a supported format: .vox (MagicaVoxel), .obj (Wavefront), .fbx (Autodesk), or primitive integer/point.")]
    AssetLoadError {
        path: String,
        details: String,
    },

    #[error("Failed to load Gobo / Light Cookie texture '{path}': {details}\n  Hint: Provide a valid PNG, JPG, or HDR image file with clear pattern contrast.")]
    GoboLoadError {
        path: String,
        details: String,
    },

    #[error("GPU Vulkan initialization failed: {details}\n  Hint: Check that your GPU drivers and Vulkan loader are properly installed.")]
    GpuInitError {
        details: String,
    },

    #[error("Window creation failed: {details}")]
    WindowError {
        details: String,
    },
}

pub type Result<T> = std::result::Result<T, AzteriskError>;
