use crate::audio::{AudioManager, SpatialSound};
use crate::camera::{Camera, CameraMode};
use crate::entity::Entity;
use crate::errors::{AzteriskError, Result};
use crate::lighting::{Light, LightType, LightsUniform, MAX_LIGHTS};
use crate::renderer::gpu::GpuContext;
use crate::renderer::pipeline::{CameraUniform, ModelUniform, Pipeline2D, Pipeline3D};
use crate::spatial::{IntoChunkCoord3D, IntoSubCoord3D, SubCoord3D};
use crate::text::FontAtlas;
use crate::ui::UIElement;
use glam::Mat4;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use wgpu::util::DeviceExt;
use wgpu::*;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, KeyEvent, WindowEvent};
pub use winit::event::ElementState;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
pub use winit::keyboard::KeyCode;
use winit::window::{Window, WindowAttributes, WindowId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    TwoD = 0,
    ThreeD = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

pub struct Scene {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub title: String,
    pub mode: RenderMode,
    pub world_span: i32,
    pub subdivisions: i32,

    pub cameras: HashMap<String, Camera>,
    pub active_camera: String,

    pub lights: Vec<Light>,
    pub entities: Vec<Entity>,

    pub audio: AudioManager,
    pub ui_elements: Vec<UIElement>,

    pub next_entity_id: usize,
    pub on_mouse_click: Option<Box<dyn FnMut(&mut Scene, MouseButton) + 'static>>,
    pub on_update: Option<Box<dyn FnMut(&mut Scene, f32) + 'static>>,
    pub on_key: Option<Box<dyn FnMut(&mut Scene, KeyCode, ElementState) + 'static>>,
    pub pressed_keys: HashSet<KeyCode>,
    pub entities_cleared: bool,
}

impl Scene {
    /// Defines a new scene and world coordinate space.
    /// - `mode`: 1 for 3D, 0 for 2D
    /// - `world_span`: macro-chunk bounds [-world_span ..= world_span] (e.g. 100)
    /// - `subdivisions`: micro-cell bounds [-subdivisions ..= subdivisions] (e.g. 64)
    pub fn define(
        width: u32,
        height: u32,
        fps: u32,
        title: impl Into<String>,
        mode: i32,
        world_span: i32,
        subdivisions: i32,
    ) -> Self {
        let render_mode = if mode == 0 {
            RenderMode::TwoD
        } else {
            RenderMode::ThreeD
        };

        Self {
            width,
            height,
            fps: fps.max(1),
            title: title.into(),
            mode: render_mode,
            world_span: world_span.max(1),
            subdivisions: subdivisions.max(1),
            cameras: HashMap::new(),
            active_camera: String::new(),
            lights: Vec::new(),
            entities: Vec::new(),
            audio: AudioManager::new(),
            ui_elements: Vec::new(),
            next_entity_id: 1,
            on_mouse_click: None,
            on_update: None,
            on_key: None,
            pressed_keys: HashSet::new(),
            entities_cleared: false,
        }
    }

    /// Registers a continuous frame update callback called every frame with delta time (seconds).
    pub fn on_update<F>(&mut self, callback: F) -> &mut Self
    where
        F: FnMut(&mut Scene, f32) + 'static,
    {
        self.on_update = Some(Box::new(callback));
        self
    }

    /// Registers a keyboard event callback triggered when keys are pressed or released.
    pub fn on_key<F>(&mut self, callback: F) -> &mut Self
    where
        F: FnMut(&mut Scene, KeyCode, ElementState) + 'static,
    {
        self.on_key = Some(Box::new(callback));
        self
    }

    /// Queries whether a physical key is currently held down.
    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }

    /// Removes all existing entities and prepares the GPU buffer cache for reload.
    pub fn clear_entities(&mut self) {
        self.entities.clear();
        self.entities_cleared = true;
    }

    /// Removes an entity by ID, returning it if found.
    pub fn remove_entity(&mut self, id: usize) -> Option<Entity> {
        if let Some(pos) = self.entities.iter().position(|e| e.id == id) {
            let removed = self.entities.remove(pos);
            self.entities_cleared = true;
            Some(removed)
        } else {
            None
        }
    }

    /// Clears all 2D UI elements.
    pub fn clear_ui(&mut self) {
        self.ui_elements.clear();
    }

    /// Registers a scene-level mouse click callback triggered when clicking inside the 3D world.
    pub fn on_mouse_click<F>(&mut self, callback: F) -> &mut Self
    where
        F: FnMut(&mut Scene, MouseButton) + 'static,
    {
        self.on_mouse_click = Some(Box::new(callback));
        self
    }

    /// Retrieves a reference to the active camera.
    pub fn active_cam(&self) -> &Camera {
        self.cameras
            .get(&self.active_camera)
            .expect("Scene has no active camera configured")
    }

    /// Retrieves a mutable reference to the active camera.
    pub fn active_cam_mut(&mut self) -> &mut Camera {
        self.cameras
            .get_mut(&self.active_camera)
            .expect("Scene has no active camera configured")
    }

    /// Finds an entity by its unique runtime identifier.
    pub fn find_entity(&self, id: usize) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == id)
    }

    /// Finds a mutable reference to an entity by its unique runtime identifier.
    pub fn find_entity_mut(&mut self, id: usize) -> Option<&mut Entity> {
        self.entities.iter_mut().find(|e| e.id == id)
    }

    /// Defines or retrieves a camera anchored at a grid chunk.
    pub fn camera(&mut self, name: impl Into<String>, chunk_coord: impl IntoChunkCoord3D) -> &mut Camera {
        let name_str = name.into();
        let chunk = chunk_coord.into_chunk_coord().unwrap_or_default();
        let sub = SubCoord3D::new(0, 0, 0);

        if self.active_camera.is_empty() {
            self.active_camera = name_str.clone();
        }

        let subdivisions = self.subdivisions;
        self.cameras
            .entry(name_str.clone())
            .or_insert_with(|| Camera::new(name_str.clone(), chunk, sub, subdivisions));

        self.cameras.get_mut(&name_str).unwrap()
    }

    /// Adds a light to the scene.
    pub fn light(
        &mut self,
        name: impl Into<String>,
        light_type: LightType,
        chunk_coord: impl IntoChunkCoord3D,
        sub_coord: impl IntoSubCoord3D,
    ) -> &mut Light {
        let chunk = chunk_coord.into_chunk_coord().unwrap_or_default();
        let sub = sub_coord.into_sub_coord().unwrap_or_default();

        if let Err(e) = chunk.validate(self.world_span) {
            eprintln!("[azterisk_render warning] {e}");
        }
        if let Err(e) = sub.validate(self.subdivisions) {
            eprintln!("[azterisk_render warning] {e}");
        }

        let l = Light::new(name, light_type, chunk, sub, self.subdivisions);
        self.lights.push(l);
        self.lights.last_mut().unwrap()
    }

    /// Adds an element to the world.
    /// `asset` can be:
    /// - `1` or `"point"`: a unit voxel cube perfectly occupying the cell
    /// - `"path/to/model.vox"`: MagicaVoxel model
    /// - `"path/to/model.obj"`: Wavefront OBJ model
    /// - `"path/to/model.fbx"`: Autodesk FBX model
    pub fn add(
        &mut self,
        asset: impl IntoAsset,
        chunk_coord: impl IntoChunkCoord3D,
        sub_coord: impl IntoSubCoord3D,
    ) -> &mut Entity {
        let chunk = chunk_coord
            .into_chunk_coord()
            .expect("Valid chunk coordinate required");
        let sub = sub_coord
            .into_sub_coord()
            .expect("Valid sub coordinate required");

        if let Err(e) = chunk.validate(self.world_span) {
            panic!("{e}");
        }
        if let Err(e) = sub.validate(self.subdivisions) {
            panic!("{e}");
        }

        let mesh = asset.load_mesh().unwrap_or_else(|e| {
            panic!("{e}");
        });

        let id = self.next_entity_id;
        self.next_entity_id += 1;

        let entity = Entity::new(id, chunk, sub, mesh, self.subdivisions);
        self.entities.push(entity);
        self.entities.last_mut().unwrap()
    }

    /// Anchors a 3D spatial sound to a coordinate grid cell.
    pub fn sound(
        &mut self,
        path: impl Into<String>,
        chunk_coord: impl IntoChunkCoord3D,
        sub_coord: impl IntoSubCoord3D,
    ) -> &mut SpatialSound {
        self.audio.add_spatial_sound(path, chunk_coord, sub_coord, self.subdivisions)
    }

    /// Plays 2D background music with volume and looping options.
    pub fn play_music(&mut self, path: impl Into<String>, looping: bool, volume: f32) -> &mut Self {
        self.audio.play_music(path, looping, volume);
        self
    }

    /// Plays an instantaneous 2D sound effect (e.g. button click, chime).
    pub fn play_sfx(&self, path: impl Into<String>, volume: f32) {
        self.audio.play_sfx(path, volume);
    }

    /// Adds an interactive UI element (button, label, container) to the scene overlay.
    pub fn add_ui(&mut self, element: UIElement) -> &mut UIElement {
        self.ui_elements.push(element);
        self.ui_elements.last_mut().unwrap()
    }

    /// Launches the window and starts the render loop.
    pub fn run(mut self) {
        // 1. Diagnostic Fallback: Ensure camera exists
        if self.cameras.is_empty() {
            println!(
                "[azterisk_render] Note: No camera was defined. Creating default 'Player' camera at chunk 0x0x0."
            );
            self.camera("Player", "0x0x0");
        }

        // 2. Diagnostic Fallback: Ensure lighting exists in 3D mode
        if self.mode == RenderMode::ThreeD && self.lights.is_empty() {
            println!(
                "[azterisk_render] Note: No lights were defined. Adding default Sun light so geometry is visible."
            );
            self.light("Sun", LightType::Directional, "0x0x0", "0,10,0")
                .direction(-0.4, -1.0, -0.3)
                .color([1.0, 0.98, 0.92])
                .intensity(12.0);
        }

        let event_loop = EventLoop::new().expect("Failed to create event loop");
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut app = SceneApp::new(self);
        event_loop.run_app(&mut app).expect("Event loop error");
    }
}

/// Helper trait to convert integers, string identifiers, or file paths into Meshes.
pub trait IntoAsset {
    fn load_mesh(self) -> Result<crate::entity::Mesh>;
}

impl IntoAsset for i32 {
    fn load_mesh(self) -> Result<crate::entity::Mesh> {
        Ok(crate::entity::Mesh::unit_cube(glam::Vec4::ONE))
    }
}

impl IntoAsset for crate::entity::Mesh {
    fn load_mesh(self) -> Result<crate::entity::Mesh> {
        Ok(self)
    }
}

impl IntoAsset for &str {
    fn load_mesh(self) -> Result<crate::entity::Mesh> {
        let trimmed = self.trim();
        if trimmed == "point" || trimmed == "cube" || trimmed == "1" {
            Ok(crate::entity::Mesh::unit_cube(glam::Vec4::ONE))
        } else if trimmed.ends_with(".vox") {
            crate::entity::Mesh::from_vox(trimmed)
        } else if trimmed.ends_with(".obj") {
            crate::entity::Mesh::from_obj(trimmed)
        } else if trimmed.ends_with(".fbx") {
            crate::entity::Mesh::from_fbx(trimmed)
        } else {
            Err(AzteriskError::AssetLoadError {
                path: trimmed.to_string(),
                details: "Unknown asset format. Supported: .vox, .obj, .fbx, or 'point'/'1'.".to_string(),
            })
        }
    }
}

struct SceneApp {
    scene: Scene,
    gpu: Option<GpuContext>,
    pipeline_3d: Option<Pipeline3D>,
    pipeline_2d: Option<Pipeline2D>,

    mesh_buffers: Vec<(Buffer, Buffer, u32)>,
    gobo_bind_groups: Vec<Option<BindGroup>>,
    player_model_buffer: Option<(Buffer, Buffer, u32)>,

    font_atlas: Option<FontAtlas>,
    font_bind_group: Option<BindGroup>,

    last_frame_time: Instant,
    target_frame_duration: Duration,
}

impl SceneApp {
    fn new(scene: Scene) -> Self {
        let target_frame_duration = Duration::from_micros(1_000_000 / scene.fps as u64);
        let font_atlas = FontAtlas::new(crate::text::DEFAULT_FONT_BYTES, 48.0).ok();
        Self {
            scene,
            gpu: None,
            pipeline_3d: None,
            pipeline_2d: None,
            mesh_buffers: Vec::new(),
            gobo_bind_groups: Vec::new(),
            player_model_buffer: None,
            font_atlas,
            font_bind_group: None,
            last_frame_time: Instant::now(),
            target_frame_duration,
        }
    }

    fn init_gpu(&mut self, window: Arc<Window>) {
        let width = self.scene.width;
        let height = self.scene.height;

        let gpu = pollster::block_on(GpuContext::init(window, width, height))
            .expect("Failed to initialize GPU context via Vulkan");

        let p3d = Pipeline3D::new(&gpu);
        let p2d = Pipeline2D::new(&gpu);

        let mut mesh_buffers = Vec::new();
        for ent in &self.scene.entities {
            let vbo = gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("VBO entity {}", ent.id)),
                    contents: bytemuck::cast_slice(&ent.mesh.vertices),
                    usage: BufferUsages::VERTEX,
                });
            let ibo = gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("IBO entity {}", ent.id)),
                    contents: bytemuck::cast_slice(&ent.mesh.indices),
                    usage: BufferUsages::INDEX,
                });
            mesh_buffers.push((vbo, ibo, ent.mesh.indices.len() as u32));
        }

        let mut gobo_bind_groups = Vec::new();
        for light in &self.scene.lights {
            if let Some(img) = &light.gobo_image {
                let (view, sampler) = gpu.load_texture_from_image(img, &light.name);
                let bg = p3d.create_gobo_bind_group(&gpu.device, &view, &sampler);
                gobo_bind_groups.push(Some(bg));
            } else {
                gobo_bind_groups.push(None);
            }
        }

        // Upload player model buffer for 3rd person mode
        let player_mesh = self
            .scene
            .cameras
            .get(&self.scene.active_camera)
            .and_then(|c| c.attached_model.clone())
            .unwrap_or_else(|| {
                crate::entity::Mesh::unit_cube(glam::Vec4::new(0.25, 0.65, 1.0, 1.0))
            });

        let player_vbo = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Player Model VBO"),
                contents: bytemuck::cast_slice(&player_mesh.vertices),
                usage: BufferUsages::VERTEX,
            });
        let player_ibo = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Player Model IBO"),
                contents: bytemuck::cast_slice(&player_mesh.indices),
                usage: BufferUsages::INDEX,
            });
        let player_model_buffer = Some((player_vbo, player_ibo, player_mesh.indices.len() as u32));

        // Upload Font Atlas texture for 2D UI text rendering
        let font_bind_group = self.font_atlas.as_ref().map(|atlas| {
            let (view, sampler) = gpu.load_r8_texture(
                atlas.atlas_width,
                atlas.atlas_height,
                &atlas.atlas_data,
                "UI Font Atlas",
            );
            p2d.create_font_bind_group(&gpu.device, &view, &sampler)
        });

        self.gpu = Some(gpu);
        self.pipeline_3d = Some(p3d);
        self.pipeline_2d = Some(p2d);
        self.mesh_buffers = mesh_buffers;
        self.gobo_bind_groups = gobo_bind_groups;
        self.player_model_buffer = player_model_buffer;
        self.font_bind_group = font_bind_group;
    }

    fn render(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame_time);
        if elapsed < self.target_frame_duration {
            return;
        }
        let dt = elapsed.as_secs_f32().min(0.1);
        self.last_frame_time = now;

        if let Some(cam) = self.scene.cameras.get_mut(&self.scene.active_camera) {
            cam.update(dt);
            self.scene.audio.update_listener(
                cam.eye_position(),
                cam.forward(),
                cam.right(),
            );
        }

        // Trigger continuous frame update callback
        if let Some(mut cb) = self.scene.on_update.take() {
            cb(&mut self.scene, dt);
            self.scene.on_update = Some(cb);
        }

        let gpu = match &self.gpu {
            Some(g) => g,
            None => return,
        };

        let output = match gpu.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(texture) => texture,
            CurrentSurfaceTexture::Suboptimal(texture) => texture,
            _ => return,
        };

        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = gpu
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Scene Render Encoder"),
            });

        // Handle entity removals / stage clears
        if self.scene.entities_cleared {
            self.mesh_buffers.clear();
            self.scene.entities_cleared = false;
        }

        if self.mesh_buffers.len() > self.scene.entities.len() {
            self.mesh_buffers.truncate(self.scene.entities.len());
        }

        // Synchronize any dynamically modified entity meshes with GPU buffers
        for (i, ent) in self.scene.entities.iter_mut().enumerate() {
            if ent.is_mesh_dirty || i >= self.mesh_buffers.len() {
                let vbo = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("Dynamic VBO entity {}", ent.id)),
                    contents: bytemuck::cast_slice(&ent.mesh.vertices),
                    usage: BufferUsages::VERTEX,
                });
                let ibo = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("Dynamic IBO entity {}", ent.id)),
                    contents: bytemuck::cast_slice(&ent.mesh.indices),
                    usage: BufferUsages::INDEX,
                });
                if i < self.mesh_buffers.len() {
                    self.mesh_buffers[i] = (vbo, ibo, ent.mesh.indices.len() as u32);
                } else {
                    self.mesh_buffers.push((vbo, ibo, ent.mesh.indices.len() as u32));
                }
                ent.is_mesh_dirty = false;
            }
        }

        let aspect = gpu.config.width as f32 / gpu.config.height as f32;

        if self.scene.mode == RenderMode::ThreeD {
            let p3d = self.pipeline_3d.as_ref().unwrap();

            if let Some(cam) = self.scene.cameras.get(&self.scene.active_camera) {
                let view_proj = cam.projection_matrix(aspect) * cam.view_matrix();
                let eye = cam.eye_position();
                let cam_uniform = CameraUniform {
                    view_proj: view_proj.to_cols_array_2d(),
                    camera_pos: [eye.x, eye.y, eye.z, 1.0],
                };
                gpu.queue.write_buffer(
                    &p3d.camera_buffer,
                    0,
                    bytemuck::cast_slice(&[cam_uniform]),
                );
            }

            let mut lights_uniform = LightsUniform::default();
            let count = self.scene.lights.len().min(MAX_LIGHTS);
            lights_uniform.count = count as u32;
            for (i, light) in self.scene.lights.iter().take(MAX_LIGHTS).enumerate() {
                lights_uniform.lights[i] = light.to_uniform();
            }
            gpu.queue.write_buffer(
                &p3d.light_buffer,
                0,
                bytemuck::cast_slice(&[lights_uniform]),
            );

            {
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("3D Render Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color {
                                r: 0.05,
                                g: 0.05,
                                b: 0.07,
                                a: 1.0,
                            }),
                            store: StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                        view: &gpu.depth_texture_view,
                        depth_ops: Some(Operations {
                            load: LoadOp::Clear(1.0),
                            store: StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

                render_pass.set_pipeline(&p3d.render_pipeline);
                render_pass.set_bind_group(0, &p3d.global_bind_group, &[]);

                let gobo_bg = self
                    .gobo_bind_groups
                    .first()
                    .and_then(|bg| bg.as_ref())
                    .unwrap_or(&p3d.default_gobo_bind_group);
                render_pass.set_bind_group(1, gobo_bg, &[]);

                // Render scene entities
                for (i, ent) in self.scene.entities.iter().enumerate() {
                    if !ent.visible {
                        continue;
                    }
                    if let Some((vbo, ibo, index_count)) = self.mesh_buffers.get(i) {
                        let model_mat = ent.model_matrix();
                        let m_uniform = ModelUniform {
                            model_matrix: model_mat.to_cols_array_2d(),
                        };
                        gpu.queue.write_buffer(
                            &p3d.model_buffer,
                            0,
                            bytemuck::cast_slice(&[m_uniform]),
                        );

                        render_pass.set_vertex_buffer(0, vbo.slice(..));
                        render_pass.set_index_buffer(ibo.slice(..), IndexFormat::Uint32);
                        render_pass.draw_indexed(0..*index_count, 0, 0..1);
                    }
                }

                // Render Player Model in Third Person Mode
                if let Some(cam) = self.scene.cameras.get(&self.scene.active_camera) {
                    if cam.mode == CameraMode::ThirdPerson {
                        if let Some((vbo, ibo, index_count)) = &self.player_model_buffer {
                            let player_mat = cam.player_model_matrix();
                            let m_uniform = ModelUniform {
                                model_matrix: player_mat.to_cols_array_2d(),
                            };
                            gpu.queue.write_buffer(
                                &p3d.model_buffer,
                                0,
                                bytemuck::cast_slice(&[m_uniform]),
                            );

                            render_pass.set_vertex_buffer(0, vbo.slice(..));
                            render_pass.set_index_buffer(ibo.slice(..), IndexFormat::Uint32);
                            render_pass.draw_indexed(0..*index_count, 0, 0..1);
                        }
                    }
                }
            }
        } else {
            // 2D Orthographic Mode
            let p2d = self.pipeline_2d.as_ref().unwrap();
            let half_w = self.scene.width as f32 / 2.0;
            let half_h = self.scene.height as f32 / 2.0;
            #[allow(deprecated)]
            let ortho = Mat4::orthographic_rh_gl(-half_w, half_w, -half_h, half_h, -100.0, 100.0);

            let cam_uniform = CameraUniform {
                view_proj: ortho.to_cols_array_2d(),
                camera_pos: [0.0, 0.0, 0.0, 1.0],
            };
            gpu.queue.write_buffer(
                &p2d.camera_buffer,
                0,
                bytemuck::cast_slice(&[cam_uniform]),
            );

            {
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("2D Render Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color {
                                r: 0.08,
                                g: 0.08,
                                b: 0.10,
                                a: 1.0,
                            }),
                            store: StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

                render_pass.set_pipeline(&p2d.render_pipeline);
                render_pass.set_bind_group(0, &p2d.bind_group, &[]);
                render_pass.set_bind_group(1, &p2d.default_font_bind_group, &[]);

                for (i, ent) in self.scene.entities.iter().enumerate() {
                    if !ent.visible {
                        continue;
                    }
                    if let Some((vbo, ibo, index_count)) = self.mesh_buffers.get(i) {
                        let model_mat = ent.model_matrix();
                        let m_uniform = ModelUniform {
                            model_matrix: model_mat.to_cols_array_2d(),
                        };
                        gpu.queue.write_buffer(
                            &p2d.model_buffer,
                            0,
                            bytemuck::cast_slice(&[m_uniform]),
                        );

                        render_pass.set_vertex_buffer(0, vbo.slice(..));
                        render_pass.set_index_buffer(ibo.slice(..), IndexFormat::Uint32);
                        render_pass.draw_indexed(0..*index_count, 0, 0..1);
                    }
                }
            }
        }

        // Render UI Overlay with Typography
        if !self.scene.ui_elements.is_empty() {
            let p2d = self.pipeline_2d.as_ref().unwrap();
            let half_w = gpu.config.width as f32 / 2.0;
            let half_h = gpu.config.height as f32 / 2.0;
            #[allow(deprecated)]
            let ui_ortho = Mat4::orthographic_rh_gl(-half_w, half_w, -half_h, half_h, -100.0, 100.0);

            let cam_uniform = CameraUniform {
                view_proj: ui_ortho.to_cols_array_2d(),
                camera_pos: [0.0, 0.0, 0.0, 1.0],
            };
            gpu.queue.write_buffer(
                &p2d.camera_buffer,
                0,
                bytemuck::cast_slice(&[cam_uniform]),
            );

            let model_uniform = ModelUniform {
                model_matrix: Mat4::IDENTITY.to_cols_array_2d(),
            };
            gpu.queue.write_buffer(
                &p2d.model_buffer,
                0,
                bytemuck::cast_slice(&[model_uniform]),
            );

            let mut all_vertices = Vec::new();
            let mut all_indices = Vec::new();
            for el in &self.scene.ui_elements {
                let mesh = el.generate_mesh_with_font(
                    gpu.config.width as f32,
                    gpu.config.height as f32,
                    self.font_atlas.as_ref(),
                );
                let base_idx = all_vertices.len() as u32;
                all_vertices.extend_from_slice(&mesh.vertices);
                for idx in mesh.indices {
                    all_indices.push(base_idx + idx);
                }
            }

            if !all_indices.is_empty() {
                let ui_vbo = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("UI VBO"),
                    contents: bytemuck::cast_slice(&all_vertices),
                    usage: BufferUsages::VERTEX,
                });
                let ui_ibo = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("UI IBO"),
                    contents: bytemuck::cast_slice(&all_indices),
                    usage: BufferUsages::INDEX,
                });

                let mut ui_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("UI Overlay Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

                ui_pass.set_pipeline(&p2d.render_pipeline);
                ui_pass.set_bind_group(0, &p2d.bind_group, &[]);
                let font_bg = self
                    .font_bind_group
                    .as_ref()
                    .unwrap_or(&p2d.default_font_bind_group);
                ui_pass.set_bind_group(1, font_bg, &[]);
                ui_pass.set_vertex_buffer(0, ui_vbo.slice(..));
                ui_pass.set_index_buffer(ui_ibo.slice(..), IndexFormat::Uint32);
                ui_pass.draw_indexed(0..all_indices.len() as u32, 0, 0..1);
            }
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));
        gpu.queue.present(output);
    }
}

impl ApplicationHandler for SceneApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_none() {
            let win_attrs = WindowAttributes::default()
                .with_title(&self.scene.title)
                .with_inner_size(PhysicalSize::new(self.scene.width, self.scene.height));

            let window = Arc::new(
                event_loop
                    .create_window(win_attrs)
                    .expect("Failed to create window"),
            );
            self.init_gpu(window);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(new_size.width, new_size.height);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let mouse_x = position.x as f32;
                let mouse_y = position.y as f32;
                if let Some(gpu) = &self.gpu {
                    let sw = gpu.config.width as f32;
                    let sh = gpu.config.height as f32;
                    for el in &mut self.scene.ui_elements {
                        el.is_hovered = el.hit_test(mouse_x, mouse_y, sw, sh);
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                let mut ui_handled = false;
                if button == winit::event::MouseButton::Left {
                    let mut clicked_sound = None;
                    for el in &mut self.scene.ui_elements {
                        if el.is_hovered {
                            ui_handled = true;
                            if let Some(snd) = &el.sound_path {
                                clicked_sound = Some(snd.clone());
                            }
                            if let Some(mut cb) = el.on_click.take() {
                                cb(el);
                                el.on_click = Some(cb);
                            }
                        }
                    }
                    if let Some(snd) = clicked_sound {
                        self.scene.audio.play_sfx(snd, 1.0);
                    }
                }

                if !ui_handled {
                    let mb = match button {
                        winit::event::MouseButton::Left => MouseButton::Left,
                        winit::event::MouseButton::Right => MouseButton::Right,
                        winit::event::MouseButton::Middle => MouseButton::Middle,
                        winit::event::MouseButton::Back => MouseButton::Other(1),
                        winit::event::MouseButton::Forward => MouseButton::Other(2),
                        winit::event::MouseButton::Other(x) => MouseButton::Other(x),
                    };
                    if let Some(mut cb) = self.scene.on_mouse_click.take() {
                        cb(&mut self.scene, mb);
                        self.scene.on_mouse_click = Some(cb);
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key,
                        state,
                        ..
                    },
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                if let PhysicalKey::Code(code) = physical_key {
                    if pressed {
                        self.scene.pressed_keys.insert(code);
                    } else {
                        self.scene.pressed_keys.remove(&code);
                    }

                    if let Some(mut cb) = self.scene.on_key.take() {
                        cb(&mut self.scene, code, state);
                        self.scene.on_key = Some(cb);
                    }
                }

                if pressed {
                    let span = self.scene.world_span;
                    let subs = self.scene.subdivisions;
                    match physical_key {
                        PhysicalKey::Code(KeyCode::KeyW) => {
                            if self.scene.mode == RenderMode::ThreeD {
                                if let Some(cam) = self.scene.cameras.get_mut(&self.scene.active_camera) {
                                    let _ = cam.step(0, 0, -1, span, subs);
                                }
                            }
                        }
                        PhysicalKey::Code(KeyCode::KeyS) => {
                            if self.scene.mode == RenderMode::ThreeD {
                                if let Some(cam) = self.scene.cameras.get_mut(&self.scene.active_camera) {
                                    let _ = cam.step(0, 0, 1, span, subs);
                                }
                            }
                        }
                        PhysicalKey::Code(KeyCode::KeyA) => {
                            if self.scene.mode == RenderMode::ThreeD {
                                if let Some(cam) = self.scene.cameras.get_mut(&self.scene.active_camera) {
                                    let _ = cam.step(-1, 0, 0, span, subs);
                                }
                            }
                        }
                        PhysicalKey::Code(KeyCode::KeyD) => {
                            if self.scene.mode == RenderMode::ThreeD {
                                if let Some(cam) = self.scene.cameras.get_mut(&self.scene.active_camera) {
                                    let _ = cam.step(1, 0, 0, span, subs);
                                }
                            }
                        }
                        PhysicalKey::Code(KeyCode::Space) => {
                            if self.scene.mode == RenderMode::ThreeD {
                                if let Some(cam) = self.scene.cameras.get_mut(&self.scene.active_camera) {
                                    let _ = cam.step(0, 1, 0, span, subs);
                                }
                            }
                        }
                        PhysicalKey::Code(KeyCode::ShiftLeft) => {
                            if self.scene.mode == RenderMode::ThreeD {
                                if let Some(cam) = self.scene.cameras.get_mut(&self.scene.active_camera) {
                                    let _ = cam.step(0, -1, 0, span, subs);
                                }
                            }
                        }
                        // Toggle between First Person & Third Person on V or F5!
                        PhysicalKey::Code(KeyCode::KeyV) | PhysicalKey::Code(KeyCode::F5) => {
                            if let Some(cam) = self.scene.cameras.get_mut(&self.scene.active_camera) {
                                cam.toggle_perspective();
                                println!("[azterisk_render] Camera mode toggled to: {:?}", cam.mode);
                            }
                        }
                        PhysicalKey::Code(KeyCode::Escape) => {
                            event_loop.exit();
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
                if let Some(gpu) = &self.gpu {
                    gpu.window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            if let Some(cam) = self.scene.cameras.get_mut(&self.scene.active_camera) {
                cam.rotate(dx as f32, dy as f32);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(gpu) = &self.gpu {
            gpu.window.request_redraw();
        }
    }
}
