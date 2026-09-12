use crate::errors::{AzteriskError, Result};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::*;
use winit::window::Window;

pub struct GpuContext {
    pub window: Arc<Window>,
    pub surface: Surface<'static>,
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
    pub depth_texture_view: TextureView,
    pub default_white_texture_view: TextureView,
    pub default_sampler: Sampler,
}

impl GpuContext {
    pub async fn init(window: Arc<Window>, width: u32, height: u32) -> Result<Self> {
        let mut desc = InstanceDescriptor::new_without_display_handle();
        desc.backends = Backends::VULKAN | Backends::PRIMARY;
        let instance = Instance::new(desc);

        let surface = instance
            .create_surface(Arc::clone(&window))
            .map_err(|e| AzteriskError::GpuInitError {
                details: format!("Failed to create surface: {e}"),
            })?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|e| AzteriskError::GpuInitError {
                details: format!("No compatible Vulkan GPU adapter found: {e:?}"),
            })?;

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("Azterisk Device"),
                required_features: Features::empty(),
                required_limits: Limits::default(),
                memory_hints: MemoryHints::Performance,
                experimental_features: ExperimentalFeatures::default(),
                trace: Default::default(),
            })
            .await
            .map_err(|e| AzteriskError::GpuInitError {
                details: format!("Failed to request device: {e}"),
            })?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            color_space: SurfaceColorSpace::Auto,
            width: width.max(1),
            height: height.max(1),
            present_mode: PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let depth_texture_view = Self::create_depth_texture(&device, config.width, config.height);

        let white_texture = device.create_texture_with_data(
            &queue,
            &TextureDescriptor {
                label: Some("Default White Gobo Texture"),
                size: Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &[255, 255, 255, 255],
        );
        let default_white_texture_view =
            white_texture.create_view(&TextureViewDescriptor::default());

        let default_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Default Gobo Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            depth_texture_view,
            default_white_texture_view,
            default_sampler,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture_view =
                Self::create_depth_texture(&self.device, self.config.width, self.config.height);
        }
    }

    pub fn create_depth_texture(device: &Device, width: u32, height: u32) -> TextureView {
        let size = Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        };
        let desc = TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);
        texture.create_view(&TextureViewDescriptor::default())
    }

    pub fn load_texture_from_image(
        &self,
        img: &image::RgbaImage,
        label: &str,
    ) -> (TextureView, Sampler) {
        let dimensions = img.dimensions();
        let size = Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture_with_data(
            &self.queue,
            &TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            img.as_raw(),
        );

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&SamplerDescriptor {
            label: Some(&format!("{label} Sampler")),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });

        (view, sampler)
    }

    pub fn load_r8_texture(
        &self,
        width: u32,
        height: u32,
        data: &[u8],
        label: &str,
    ) -> (TextureView, Sampler) {
        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture_with_data(
            &self.queue,
            &TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R8Unorm,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            data,
        );

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&SamplerDescriptor {
            label: Some(&format!("{label} Sampler")),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });

        (view, sampler)
    }
}
