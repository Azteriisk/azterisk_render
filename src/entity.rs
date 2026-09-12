use crate::errors::{AzteriskError, Result};
use crate::lighting::IntoColor;
use crate::spatial::{grid_to_world_3d, ChunkCoord3D, SubCoord3D};
use glam::{Mat4, Quat, Vec3, Vec4};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2 + std::mem::size_of::<[f32; 2]>())
                        as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn unit_cube(color: Vec4) -> Self {
        let c = [color.x, color.y, color.z, color.w];
        let half = 0.5f32;

        let vertices = vec![
            // Front face (+Z)
            Vertex { position: [-half, -half,  half], normal: [ 0.0,  0.0,  1.0], uv: [0.0, 1.0], color: c },
            Vertex { position: [ half, -half,  half], normal: [ 0.0,  0.0,  1.0], uv: [1.0, 1.0], color: c },
            Vertex { position: [ half,  half,  half], normal: [ 0.0,  0.0,  1.0], uv: [1.0, 0.0], color: c },
            Vertex { position: [-half,  half,  half], normal: [ 0.0,  0.0,  1.0], uv: [0.0, 0.0], color: c },

            // Back face (-Z)
            Vertex { position: [ half, -half, -half], normal: [ 0.0,  0.0, -1.0], uv: [0.0, 1.0], color: c },
            Vertex { position: [-half, -half, -half], normal: [ 0.0,  0.0, -1.0], uv: [1.0, 1.0], color: c },
            Vertex { position: [-half,  half, -half], normal: [ 0.0,  0.0, -1.0], uv: [1.0, 0.0], color: c },
            Vertex { position: [ half,  half, -half], normal: [ 0.0,  0.0, -1.0], uv: [0.0, 0.0], color: c },

            // Top face (+Y)
            Vertex { position: [-half,  half,  half], normal: [ 0.0,  1.0,  0.0], uv: [0.0, 1.0], color: c },
            Vertex { position: [ half,  half,  half], normal: [ 0.0,  1.0,  0.0], uv: [1.0, 1.0], color: c },
            Vertex { position: [ half,  half, -half], normal: [ 0.0,  1.0,  0.0], uv: [1.0, 0.0], color: c },
            Vertex { position: [-half,  half, -half], normal: [ 0.0,  1.0,  0.0], uv: [0.0, 0.0], color: c },

            // Bottom face (-Y)
            Vertex { position: [-half, -half, -half], normal: [ 0.0, -1.0,  0.0], uv: [0.0, 1.0], color: c },
            Vertex { position: [ half, -half, -half], normal: [ 0.0, -1.0,  0.0], uv: [1.0, 1.0], color: c },
            Vertex { position: [ half, -half,  half], normal: [ 0.0, -1.0,  0.0], uv: [1.0, 0.0], color: c },
            Vertex { position: [-half, -half,  half], normal: [ 0.0, -1.0,  0.0], uv: [0.0, 0.0], color: c },

            // Right face (+X)
            Vertex { position: [ half, -half,  half], normal: [ 1.0,  0.0,  0.0], uv: [0.0, 1.0], color: c },
            Vertex { position: [ half, -half, -half], normal: [ 1.0,  0.0,  0.0], uv: [1.0, 1.0], color: c },
            Vertex { position: [ half,  half, -half], normal: [ 1.0,  0.0,  0.0], uv: [1.0, 0.0], color: c },
            Vertex { position: [ half,  half,  half], normal: [ 1.0,  0.0,  0.0], uv: [0.0, 0.0], color: c },

            // Left face (-X)
            Vertex { position: [-half, -half, -half], normal: [-1.0,  0.0,  0.0], uv: [0.0, 1.0], color: c },
            Vertex { position: [-half, -half,  half], normal: [-1.0,  0.0,  0.0], uv: [1.0, 1.0], color: c },
            Vertex { position: [-half,  half,  half], normal: [-1.0,  0.0,  0.0], uv: [1.0, 0.0], color: c },
            Vertex { position: [-half,  half, -half], normal: [-1.0,  0.0,  0.0], uv: [0.0, 0.0], color: c },
        ];

        let mut indices = Vec::with_capacity(36);
        for face in 0..6 {
            let base = face * 4;
            indices.extend_from_slice(&[
                base, base + 1, base + 2,
                base, base + 2, base + 3,
            ]);
        }

        Self { vertices, indices }
    }

    pub fn from_vox(path: &str) -> Result<Self> {
        let vox_data = dot_vox::load(path).map_err(|e| AzteriskError::AssetLoadError {
            path: path.to_string(),
            details: e.to_string(),
        })?;

        let mut combined = Mesh::default();
        let palette = vox_data.palette;

        for model in &vox_data.models {
            let offset_x = -(model.size.x as f32) / 2.0;
            let offset_y = -(model.size.y as f32) / 2.0;
            let offset_z = -(model.size.z as f32) / 2.0;

            for v in &model.voxels {
                let color = palette[v.i as usize];
                let r = color.r as f32 / 255.0;
                let g = color.g as f32 / 255.0;
                let b = color.b as f32 / 255.0;
                let a = if color.a == 0 { 1.0 } else { color.a as f32 / 255.0 };

                let mut cube = Mesh::unit_cube(Vec4::new(r, g, b, a));
                let pos = Vec3::new(
                    v.x as f32 + offset_x,
                    v.z as f32 + offset_z,
                    v.y as f32 + offset_y,
                );

                let base_idx = combined.vertices.len() as u32;
                for vert in &mut cube.vertices {
                    vert.position[0] += pos.x;
                    vert.position[1] += pos.y;
                    vert.position[2] += pos.z;
                }
                combined.vertices.extend(cube.vertices);
                for idx in cube.indices {
                    combined.indices.push(base_idx + idx);
                }
            }
        }

        Ok(combined)
    }

    pub fn from_obj(path: &str) -> Result<Self> {
        let (models, _) = tobj::load_obj(
            path,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        )
        .map_err(|e| AzteriskError::AssetLoadError {
            path: path.to_string(),
            details: e.to_string(),
        })?;

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for m in models {
            let mesh = &m.mesh;
            let base_idx = vertices.len() as u32;

            for i in 0..mesh.positions.len() / 3 {
                let pos = [
                    mesh.positions[i * 3],
                    mesh.positions[i * 3 + 1],
                    mesh.positions[i * 3 + 2],
                ];
                let norm = if !mesh.normals.is_empty() {
                    [
                        mesh.normals[i * 3],
                        mesh.normals[i * 3 + 1],
                        mesh.normals[i * 3 + 2],
                    ]
                } else {
                    [0.0, 1.0, 0.0]
                };
                let uv = if !mesh.texcoords.is_empty() {
                    [mesh.texcoords[i * 2], 1.0 - mesh.texcoords[i * 2 + 1]]
                } else {
                    [0.0, 0.0]
                };

                vertices.push(Vertex {
                    position: pos,
                    normal: norm,
                    uv,
                    color: [1.0, 1.0, 1.0, 1.0],
                });
            }

            for idx in &mesh.indices {
                indices.push(base_idx + *idx);
            }
        }

        Ok(Self { vertices, indices })
    }

    pub fn from_fbx(path: &str) -> Result<Self> {
        let opts = ufbx::LoadOpts {
            target_unit_meters: 1.0,
            ..Default::default()
        };
        let scene = ufbx::load_file(path, opts).map_err(|e| AzteriskError::AssetLoadError {
            path: path.to_string(),
            details: format!("{e:?}"),
        })?;

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for mesh in &scene.meshes {
            let base_idx = vertices.len() as u32;
            for vert_idx in &mesh.vertex_indices {
                let pos = mesh.vertex_position[*vert_idx as usize];
                let norm = if mesh.vertex_normal.exists {
                    mesh.vertex_normal[*vert_idx as usize]
                } else {
                    ufbx::Vec3 { x: 0.0, y: 1.0, z: 0.0 }
                };
                let uv = if mesh.vertex_uv.exists {
                    mesh.vertex_uv[*vert_idx as usize]
                } else {
                    ufbx::Vec2 { x: 0.0, y: 0.0 }
                };

                vertices.push(Vertex {
                    position: [pos.x as f32, pos.y as f32, pos.z as f32],
                    normal: [norm.x as f32, norm.y as f32, norm.z as f32],
                    uv: [uv.x as f32, uv.y as f32],
                    color: [1.0, 1.0, 1.0, 1.0],
                });
            }

            for face in &mesh.faces {
                if face.num_indices == 3 {
                    indices.push(base_idx + face.index_begin);
                    indices.push(base_idx + face.index_begin + 1);
                    indices.push(base_idx + face.index_begin + 2);
                } else if face.num_indices == 4 {
                    indices.push(base_idx + face.index_begin);
                    indices.push(base_idx + face.index_begin + 1);
                    indices.push(base_idx + face.index_begin + 2);

                    indices.push(base_idx + face.index_begin);
                    indices.push(base_idx + face.index_begin + 2);
                    indices.push(base_idx + face.index_begin + 3);
                }
            }
        }

        Ok(Self { vertices, indices })
    }
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: usize,
    pub chunk: ChunkCoord3D,
    pub sub: SubCoord3D,
    pub world_pos: Vec3,

    pub mesh: Mesh,
    pub color: Vec4,
    pub scale: Vec3,
    pub rotation: Quat,
    pub is_mesh_dirty: bool,
    pub visible: bool,
}

impl Entity {
    pub fn new(
        id: usize,
        chunk: ChunkCoord3D,
        sub: SubCoord3D,
        mesh: Mesh,
        subdivisions: i32,
    ) -> Self {
        let world_pos = grid_to_world_3d(chunk, sub, subdivisions);
        Self {
            id,
            chunk,
            sub,
            world_pos,
            mesh,
            color: Vec4::ONE,
            scale: Vec3::ONE,
            rotation: Quat::IDENTITY,
            is_mesh_dirty: false,
            visible: true,
        }
    }

    /// Replaces the entity's geometry and marks the GPU vertex/index buffers for re-upload.
    pub fn update_mesh(&mut self, new_mesh: Mesh) {
        self.mesh = new_mesh;
        self.is_mesh_dirty = true;
    }

    pub fn color(&mut self, hex_or_rgba: impl IntoColor) -> &mut Self {
        let rgb = hex_or_rgba.into_color();
        self.color = Vec4::new(rgb.x, rgb.y, rgb.z, 1.0);
        for v in &mut self.mesh.vertices {
            v.color = [self.color.x, self.color.y, self.color.z, self.color.w];
        }
        self
    }

    pub fn visible(&mut self, visible: bool) -> &mut Self {
        self.visible = visible;
        self
    }

    pub fn position(&mut self, x: f32, y: f32, z: f32) -> &mut Self {
        self.world_pos = Vec3::new(x, y, z);
        self
    }

    pub fn rotate_axis(&mut self, axis: Vec3, angle_rad: f32) -> &mut Self {
        self.rotation = Quat::from_axis_angle(axis.normalize(), angle_rad);
        self
    }

    pub fn scale(&mut self, factor: f32) -> &mut Self {
        self.scale = Vec3::splat(factor);
        self
    }

    pub fn model_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.world_pos)
    }
}
