use crate::errors::{AzteriskError, Result};
use crate::spatial::{grid_to_world_3d, ChunkCoord3D, SubCoord3D};
use glam::{Mat4, Vec3};
use image::RgbaImage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightType {
    Ambient = 0,
    Directional = 1,
    Point = 2,
    Spot = 3,
}

#[derive(Debug, Clone)]
pub struct Light {
    pub name: String,
    pub light_type: LightType,
    pub chunk: ChunkCoord3D,
    pub sub: SubCoord3D,
    pub world_pos: Vec3,

    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub radius: f32,
    pub inner_cone_deg: f32,
    pub outer_cone_deg: f32,

    pub gobo_path: Option<String>,
    pub gobo_image: Option<RgbaImage>,
}

impl Light {
    pub fn new(
        name: impl Into<String>,
        light_type: LightType,
        chunk: ChunkCoord3D,
        sub: SubCoord3D,
        subdivisions: i32,
    ) -> Self {
        let world_pos = grid_to_world_3d(chunk, sub, subdivisions);
        Self {
            name: name.into(),
            light_type,
            chunk,
            sub,
            world_pos,
            direction: Vec3::new(0.0, -1.0, 0.0),
            color: Vec3::new(1.0, 1.0, 1.0),
            intensity: 15.0,
            radius: 50.0,
            inner_cone_deg: 25.0,
            outer_cone_deg: 40.0,
            gobo_path: None,
            gobo_image: None,
        }
    }

    pub fn direction(&mut self, x: f32, y: f32, z: f32) -> &mut Self {
        let dir = Vec3::new(x, y, z);
        self.direction = if dir.length_squared() > 0.0001 {
            dir.normalize()
        } else {
            Vec3::NEG_Y
        };
        self
    }

    pub fn color(&mut self, hex_or_rgb: impl IntoColor) -> &mut Self {
        self.color = hex_or_rgb.into_color();
        self
    }

    pub fn intensity(&mut self, val: f32) -> &mut Self {
        self.intensity = val.max(0.0);
        self
    }

    pub fn radius(&mut self, val: f32) -> &mut Self {
        self.radius = val.max(0.1);
        self
    }

    pub fn cone(&mut self, inner_deg: f32, outer_deg: f32) -> &mut Self {
        self.inner_cone_deg = inner_deg.clamp(1.0, 89.0);
        self.outer_cone_deg = outer_deg.clamp(self.inner_cone_deg, 89.9);
        self
    }

    /// Attach a Gobo / Light Cookie image to mask the projected light cone.
    pub fn gobo(&mut self, path: impl Into<String>) -> Result<&mut Self> {
        let p = path.into();
        let img = image::open(&p).map_err(|e| AzteriskError::GoboLoadError {
            path: p.clone(),
            details: e.to_string(),
        })?;
        self.gobo_path = Some(p);
        self.gobo_image = Some(img.to_rgba8());
        Ok(self)
    }

    /// Computes the light view-projection matrix for projective gobo texture mapping.
    #[allow(deprecated)]
    pub fn light_view_proj(&self) -> Mat4 {
        if self.light_type != LightType::Spot {
            return Mat4::IDENTITY;
        }

        let dir = self.direction.normalize();
        let up = if dir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
        let view = Mat4::look_at_rh(self.world_pos, self.world_pos + dir, up);
        let fov = (self.outer_cone_deg * 2.0).to_radians();
        let proj = Mat4::perspective_rh(fov, 1.0, 0.1, self.radius);
        proj * view
    }

    pub fn to_uniform(&self) -> LightUniform {
        let cos_inner = (self.inner_cone_deg.to_radians() / 2.0).cos();
        let cos_outer = (self.outer_cone_deg.to_radians() / 2.0).cos();
        let has_gobo = if self.gobo_image.is_some() { 1.0 } else { 0.0 };

        LightUniform {
            position: [self.world_pos.x, self.world_pos.y, self.world_pos.z, self.radius],
            direction: [
                self.direction.x,
                self.direction.y,
                self.direction.z,
                self.light_type as u32 as f32,
            ],
            color: [self.color.x, self.color.y, self.color.z, self.intensity],
            spot_params: [cos_inner, cos_outer, has_gobo, 0.0],
            light_view_proj: self.light_view_proj().to_cols_array_2d(),
        }
    }
}

pub trait IntoColor {
    fn into_color(self) -> Vec3;
}

impl IntoColor for Vec3 {
    fn into_color(self) -> Vec3 {
        self
    }
}

impl IntoColor for [f32; 3] {
    fn into_color(self) -> Vec3 {
        Vec3::new(self[0], self[1], self[2])
    }
}

impl IntoColor for &str {
    fn into_color(self) -> Vec3 {
        parse_hex_color(self).unwrap_or(Vec3::ONE)
    }
}

pub fn parse_hex_color(hex: &str) -> Option<Vec3> {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&clean[0..2], 16).ok()? as f32 / 255.0;
    let g = u8::from_str_radix(&clean[2..4], 16).ok()? as f32 / 255.0;
    let b = u8::from_str_radix(&clean[4..6], 16).ok()? as f32 / 255.0;
    Some(Vec3::new(r, g, b))
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub position: [f32; 4],
    pub direction: [f32; 4],
    pub color: [f32; 4],
    pub spot_params: [f32; 4],
    pub light_view_proj: [[f32; 4]; 4],
}

pub const MAX_LIGHTS: usize = 16;

/// GPU uniform buffer containing multiple dynamic lights (up to 16).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightsUniform {
    pub count: u32,
    pub _padding: [u32; 3],
    pub lights: [LightUniform; MAX_LIGHTS],
}

impl Default for LightsUniform {
    fn default() -> Self {
        Self {
            count: 0,
            _padding: [0; 3],
            lights: [LightUniform::default(); MAX_LIGHTS],
        }
    }
}
