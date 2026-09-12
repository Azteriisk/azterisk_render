use crate::entity::Vertex;
use crate::lighting::LightsUniform;
use crate::renderer::gpu::GpuContext;
use glam::Mat4;
use wgpu::util::DeviceExt;
use wgpu::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelUniform {
    pub model_matrix: [[f32; 4]; 4],
}

pub struct Pipeline3D {
    pub render_pipeline: RenderPipeline,
    pub global_bind_group_layout: BindGroupLayout,
    pub gobo_bind_group_layout: BindGroupLayout,

    pub camera_buffer: Buffer,
    pub light_buffer: Buffer,
    pub model_buffer: Buffer,
    pub global_bind_group: BindGroup,

    pub default_gobo_bind_group: BindGroup,
}

impl Pipeline3D {
    pub fn new(gpu: &GpuContext) -> Self {
        let device = &gpu.device;

        // 1. Uniform Buffers
        let camera_uniform = CameraUniform {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0, 0.0, 0.0, 1.0],
        };
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let lights_uniform = LightsUniform::default();
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lights Buffer"),
            contents: bytemuck::cast_slice(&[lights_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let model_uniform = ModelUniform {
            model_matrix: Mat4::IDENTITY.to_cols_array_2d(),
        };
        let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Model Buffer"),
            contents: bytemuck::cast_slice(&[model_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        // 2. Global Bind Group Layout
        let global_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("3D Global Bind Group Layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let global_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("3D Global Bind Group"),
            layout: &global_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: light_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: model_buffer.as_entire_binding(),
                },
            ],
        });

        // 3. Gobo Bind Group Layout
        let gobo_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Gobo Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let default_gobo_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Default Gobo Bind Group"),
            layout: &gobo_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&gpu.default_white_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&gpu.default_sampler),
                },
            ],
        });

        // 4. Shader & Pipeline
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("3D Scene Shader"),
            source: ShaderSource::Wgsl(include_str!("shaders/scene3d.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("3D Pipeline Layout"),
            bind_group_layouts: &[Some(&global_bind_group_layout), Some(&gobo_bind_group_layout)],
            immediate_size: 0,
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("3D Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::desc())],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: gpu.config.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::Less),
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        Self {
            render_pipeline,
            global_bind_group_layout,
            gobo_bind_group_layout,
            camera_buffer,
            light_buffer,
            model_buffer,
            global_bind_group,
            default_gobo_bind_group,
        }
    }

    pub fn create_gobo_bind_group(
        &self,
        device: &Device,
        view: &TextureView,
        sampler: &Sampler,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("Custom Gobo Bind Group"),
            layout: &self.gobo_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        })
    }
}

pub struct Pipeline2D {
    pub render_pipeline: RenderPipeline,
    pub camera_buffer: Buffer,
    pub model_buffer: Buffer,
    pub bind_group: BindGroup,
    pub font_bind_group_layout: BindGroupLayout,
    pub default_font_bind_group: BindGroup,
}

impl Pipeline2D {
    pub fn new(gpu: &GpuContext) -> Self {
        let device = &gpu.device;

        let camera_uniform = CameraUniform {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0, 0.0, 0.0, 1.0],
        };
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("2D Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let model_uniform = ModelUniform {
            model_matrix: Mat4::IDENTITY.to_cols_array_2d(),
        };
        let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("2D Model Buffer"),
            contents: bytemuck::cast_slice(&[model_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("2D Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("2D Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: model_buffer.as_entire_binding(),
                },
            ],
        });

        let font_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("2D Font Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let default_font_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Default 2D Font Bind Group"),
            layout: &font_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&gpu.default_white_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&gpu.default_sampler),
                },
            ],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("2D Scene Shader"),
            source: ShaderSource::Wgsl(include_str!("shaders/scene2d.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("2D Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout), Some(&font_bind_group_layout)],
            immediate_size: 0,
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("2D Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::desc())],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: gpu.config.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            render_pipeline,
            camera_buffer,
            model_buffer,
            bind_group,
            font_bind_group_layout,
            default_font_bind_group,
        }
    }

    pub fn create_font_bind_group(
        &self,
        device: &Device,
        view: &TextureView,
        sampler: &Sampler,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("Custom 2D Font Bind Group"),
            layout: &self.font_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        })
    }
}
