use crate::entity::Mesh;
use crate::errors::Result;
use crate::lighting::IntoColor;
use crate::scene::IntoAsset;
use crate::spatial::{grid_to_world_3d, ChunkCoord3D, SubCoord3D};
use glam::{Mat4, Quat, Vec3, Vec4};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ease {
    Linear,
    InOutQuad,
    InOutCubic,
    OutQuad,
}

impl Ease {
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Ease::Linear => t,
            Ease::OutQuad => t * (2.0 - t),
            Ease::InOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            Ease::InOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    (t - 1.0) * (2.0 * t - 2.0) * (2.0 * t - 2.0) + 1.0
                }
            }
        }
    }
}

/// Perspective view mode: First Person or Third Person orbit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraMode {
    FirstPerson,
    ThirdPerson,
}

/// Grid stepper configuration for discrete cell-to-cell navigation.
#[derive(Debug, Clone)]
pub struct GridStepperConfig {
    pub step_size: i32,
    pub transition_time_ms: u32,
    pub easing: Ease,
}

impl Default for GridStepperConfig {
    fn default() -> Self {
        Self {
            step_size: 1,
            transition_time_ms: 150,
            easing: Ease::InOutQuad,
        }
    }
}

impl GridStepperConfig {
    pub fn step_size(&mut self, size: i32) -> &mut Self {
        self.step_size = size;
        self
    }

    pub fn transition_time_ms(&mut self, ms: u32) -> &mut Self {
        self.transition_time_ms = ms;
        self
    }

    pub fn easing(&mut self, easing: Ease) -> &mut Self {
        self.easing = easing;
        self
    }
}

/// Camera controls binding builder with head bobbing, mouse inertia, and sensitivity.
pub struct CameraControlsBuilder<'a> {
    camera: &'a mut Camera,
}

impl<'a> CameraControlsBuilder<'a> {
    pub fn new(camera: &'a mut Camera) -> Self {
        Self { camera }
    }

    pub fn grid_stepper(&mut self) -> &mut GridStepperConfig {
        &mut self.camera.stepper
    }

    pub fn mouse_look(&mut self, enabled: bool) -> &mut Self {
        self.camera.mouse_look_enabled = enabled;
        self
    }

    pub fn look_sensitivity(&mut self, sens: f32) -> &mut Self {
        self.camera.look_sensitivity = sens;
        self
    }

    /// Enables or disables realistic mouse smoothing / delay (inertia).
    /// Value ranges from 0.0 (raw instantaneous) to 0.95 (heavy filmic inertia).
    pub fn mouse_smoothing(&mut self, smoothing: f32) -> &mut Self {
        self.camera.mouse_smoothing = smoothing.clamp(0.0, 0.98);
        self
    }

    /// Enables or disables head bobbing during grid movement.
    pub fn head_bob(&mut self, enabled: bool) -> &mut Self {
        self.camera.head_bob_enabled = enabled;
        self
    }

    /// Configures vertical head bob intensity (e.g. 0.15).
    pub fn head_bob_intensity(&mut self, intensity: f32) -> &mut Self {
        self.camera.head_bob_intensity = intensity.max(0.0);
        self
    }

    /// Configures head bob cadence / frequency (e.g. 12.0).
    pub fn head_bob_frequency(&mut self, freq: f32) -> &mut Self {
        self.camera.head_bob_frequency = freq.max(0.1);
        self
    }
}

/// Advanced camera system with discrete grid-stepping, smooth interpolation,
/// realistic mouse inertia/delay, head bobbing, FOV configuration, and 3rd person mode.
#[derive(Debug, Clone)]
pub struct Camera {
    pub name: String,
    pub chunk: ChunkCoord3D,
    pub sub: SubCoord3D,

    // Smooth movement state
    pub current_world_pos: Vec3,
    pub start_world_pos: Vec3,
    pub target_world_pos: Vec3,
    pub transition_progress: f32,
    pub is_transitioning: bool,

    // Orientation & Mouse Smoothing
    pub yaw_deg: f32,
    pub pitch_deg: f32,
    pub target_yaw_deg: f32,
    pub target_pitch_deg: f32,
    pub mouse_smoothing: f32,
    pub mouse_look_enabled: bool,
    pub look_sensitivity: f32,

    // Head Bobbing
    pub head_bob_enabled: bool,
    pub head_bob_frequency: f32,
    pub head_bob_intensity: f32,
    pub head_bob_timer: f32,
    pub current_bob_offset: Vec3,

    // Perspective Mode & Third Person Settings
    pub mode: CameraMode,
    pub third_person_distance: f32,
    pub third_person_height: f32,

    // Player Model Attachment
    pub attached_model: Option<Mesh>,
    pub attached_model_color: Vec4,
    pub attached_model_scale: Vec3,

    // Projection & FOV
    pub fov_deg: f32,
    pub near: f32,
    pub far: f32,

    // Grid Controls
    pub stepper: GridStepperConfig,
}

impl Camera {
    pub fn new(name: impl Into<String>, chunk: ChunkCoord3D, sub: SubCoord3D, subdivisions: i32) -> Self {
        let initial_pos = grid_to_world_3d(chunk, sub, subdivisions);
        Self {
            name: name.into(),
            chunk,
            sub,
            current_world_pos: initial_pos,
            start_world_pos: initial_pos,
            target_world_pos: initial_pos,
            transition_progress: 1.0,
            is_transitioning: false,
            yaw_deg: -90.0,
            pitch_deg: 0.0,
            target_yaw_deg: -90.0,
            target_pitch_deg: 0.0,
            mouse_smoothing: 0.0,
            mouse_look_enabled: true,
            look_sensitivity: 0.15,
            head_bob_enabled: false,
            head_bob_frequency: 12.0,
            head_bob_intensity: 0.15,
            head_bob_timer: 0.0,
            current_bob_offset: Vec3::ZERO,
            mode: CameraMode::FirstPerson,
            third_person_distance: 5.0,
            third_person_height: 1.8,
            attached_model: None,
            attached_model_color: Vec4::ONE,
            attached_model_scale: Vec3::ONE,
            fov_deg: 65.0,
            near: 0.1,
            far: 2000.0,
            stepper: GridStepperConfig::default(),
        }
    }

    /// Sets field of view in degrees (e.g. 75.0, 90.0).
    pub fn fov(&mut self, fov_deg: f32) -> &mut Self {
        self.fov_deg = fov_deg.clamp(20.0, 140.0);
        self
    }

    /// Configures 1st person perspective mode.
    pub fn first_person(&mut self) -> &mut Self {
        self.mode = CameraMode::FirstPerson;
        self
    }

    /// Configures 3rd person perspective mode with camera distance and height offset.
    pub fn third_person(&mut self, distance: f32, height: f32) -> &mut Self {
        self.mode = CameraMode::ThirdPerson;
        self.third_person_distance = distance.max(0.5);
        self.third_person_height = height;
        self
    }

    /// Toggles between First Person and Third Person modes on the fly.
    pub fn toggle_perspective(&mut self) {
        self.mode = match self.mode {
            CameraMode::FirstPerson => CameraMode::ThirdPerson,
            CameraMode::ThirdPerson => CameraMode::FirstPerson,
        };
    }

    /// Attaches a visual player model that renders at the player's world position in 3rd person.
    pub fn attach_model(&mut self, asset: impl IntoAsset, color: impl IntoColor) -> &mut Self {
        if let Ok(mesh) = asset.load_mesh() {
            let rgb = color.into_color();
            self.attached_model = Some(mesh);
            self.attached_model_color = Vec4::new(rgb.x, rgb.y, rgb.z, 1.0);
        }
        self
    }

    /// Sets the player model scale.
    pub fn player_model_scale(&mut self, scale: f32) -> &mut Self {
        self.attached_model_scale = Vec3::splat(scale);
        self
    }

    /// Configures default controls via a closure
    pub fn bind_default_controls<F>(&mut self, configure: F) -> &mut Self
    where
        F: FnOnce(&mut CameraControlsBuilder),
    {
        let mut builder = CameraControlsBuilder::new(self);
        configure(&mut builder);
        self
    }

    /// Steps the camera to an adjacent grid block (dx, dy, dz)
    pub fn step(&mut self, dx: i32, dy: i32, dz: i32, span: i32, subdivisions: i32) -> Result<()> {
        let mut new_sub = self.sub;
        let mut new_chunk = self.chunk;

        new_sub.x += dx * self.stepper.step_size;
        new_sub.y += dy * self.stepper.step_size;
        new_sub.z += dz * self.stepper.step_size;

        // Wrap across chunk boundaries if exceeding sub-cell limits [-subdivisions..=subdivisions]
        let chunk_span_sub = subdivisions;
        let span_len = 2 * subdivisions + 1;
        while new_sub.x > chunk_span_sub {
            new_sub.x -= span_len;
            new_chunk.x += 1;
        }
        while new_sub.x < -chunk_span_sub {
            new_sub.x += span_len;
            new_chunk.x += 1;
        }

        while new_sub.y > chunk_span_sub {
            new_sub.y -= span_len;
            new_chunk.y += 1;
        }
        while new_sub.y < -chunk_span_sub {
            new_sub.y += span_len;
            new_chunk.y += 1;
        }

        while new_sub.z > chunk_span_sub {
            new_sub.z -= span_len;
            new_chunk.z += 1;
        }
        while new_sub.z < -chunk_span_sub {
            new_sub.z += span_len;
            new_chunk.z += 1;
        }

        new_chunk.validate(span)?;

        self.chunk = new_chunk;
        self.sub = new_sub;
        self.start_world_pos = self.current_world_pos;
        self.target_world_pos = grid_to_world_3d(self.chunk, self.sub, subdivisions);
        self.transition_progress = 0.0;
        self.is_transitioning = true;

        Ok(())
    }

    /// Updates smooth position interpolation, mouse smoothing inertia, and head bobbing.
    pub fn update(&mut self, dt_seconds: f32) {
        // 1. Grid cell translation interpolation
        if self.is_transitioning {
            let duration = (self.stepper.transition_time_ms as f32 / 1000.0).max(0.001);
            self.transition_progress += dt_seconds / duration;

            if self.transition_progress >= 1.0 {
                self.transition_progress = 1.0;
                self.current_world_pos = self.target_world_pos;
                self.is_transitioning = false;
            } else {
                let factor = self.stepper.easing.apply(self.transition_progress);
                self.current_world_pos = self.start_world_pos.lerp(self.target_world_pos, factor);
            }
        }

        // 2. Mouse look smoothing (inertia delay)
        if self.mouse_smoothing > 0.001 {
            let blend = 1.0 - (1.0 - self.mouse_smoothing).powf(dt_seconds * 60.0);
            self.yaw_deg += (self.target_yaw_deg - self.yaw_deg) * blend;
            self.pitch_deg += (self.target_pitch_deg - self.pitch_deg) * blend;
        } else {
            self.yaw_deg = self.target_yaw_deg;
            self.pitch_deg = self.target_pitch_deg;
        }

        // 3. Head Bobbing animation
        if self.head_bob_enabled && self.is_transitioning {
            self.head_bob_timer += dt_seconds * self.head_bob_frequency;
            // Vertical bob: absolute sine creates footstep bounce cadence
            let bob_y = self.head_bob_timer.sin().abs() * self.head_bob_intensity;
            // Horizontal sway: gentle side-to-side shift at half frequency
            let bob_x = (self.head_bob_timer * 0.5).sin() * (self.head_bob_intensity * 0.4);
            self.current_bob_offset = Vec3::new(bob_x, bob_y, 0.0);
        } else {
            // Smoothly decay back to resting eye position when stopped
            let decay_speed = (dt_seconds * 8.0).min(1.0);
            self.current_bob_offset = self.current_bob_offset.lerp(Vec3::ZERO, decay_speed);
        }
    }

    /// Records raw mouse delta into smoothed orientation target
    pub fn rotate(&mut self, delta_x: f32, delta_y: f32) {
        if !self.mouse_look_enabled {
            return;
        }
        self.target_yaw_deg += delta_x * self.look_sensitivity;
        self.target_pitch_deg -= delta_y * self.look_sensitivity;
        self.target_pitch_deg = self.target_pitch_deg.clamp(-89.0, 89.0);

        if self.mouse_smoothing <= 0.001 {
            self.yaw_deg = self.target_yaw_deg;
            self.pitch_deg = self.target_pitch_deg;
        }
    }

    pub fn forward(&self) -> Vec3 {
        let yaw = self.yaw_deg.to_radians();
        let pitch = self.pitch_deg.to_radians();
        Vec3::new(
            yaw.cos() * pitch.cos(),
            pitch.sin(),
            yaw.sin() * pitch.cos(),
        )
        .normalize()
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    /// Returns the active eye position in world space.
    pub fn eye_position(&self) -> Vec3 {
        match self.mode {
            CameraMode::FirstPerson => {
                // Eye is at world position + head bob + natural eye height (0.5)
                self.current_world_pos + self.current_bob_offset + Vec3::new(0.0, 0.5, 0.0)
            }
            CameraMode::ThirdPerson => {
                // Eye orbits behind the player's center
                let player_center = self.current_world_pos + Vec3::new(0.0, 0.5, 0.0);
                let fwd = self.forward();
                player_center - fwd * self.third_person_distance + Vec3::new(0.0, self.third_person_height, 0.0)
            }
        }
    }

    /// Calculates the player model matrix to render the character in 3rd person mode.
    pub fn player_model_matrix(&self) -> Mat4 {
        let yaw_rad = -self.yaw_deg.to_radians() - std::f32::consts::FRAC_PI_2;
        let rotation = Quat::from_rotation_y(yaw_rad);
        Mat4::from_scale_rotation_translation(
            self.attached_model_scale,
            rotation,
            self.current_world_pos,
        )
    }

    #[allow(deprecated)]
    pub fn view_matrix(&self) -> Mat4 {
        let eye = self.eye_position();
        match self.mode {
            CameraMode::FirstPerson => {
                Mat4::look_at_rh(eye, eye + self.forward(), Vec3::Y)
            }
            CameraMode::ThirdPerson => {
                let target = self.current_world_pos + Vec3::new(0.0, 0.5, 0.0);
                Mat4::look_at_rh(eye, target, Vec3::Y)
            }
        }
    }

    #[allow(deprecated)]
    pub fn projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov_deg.to_radians(), aspect_ratio, self.near, self.far)
    }
}
