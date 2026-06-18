use std::sync::Arc;

use thiserror::Error;
use wgpu::{Backends, DeviceDescriptor, ExperimentalFeatures, Instance, InstanceDescriptor, PowerPreference, RequestAdapterOptions, SurfaceConfiguration, TextureUsages};
use winit::{dpi::PhysicalSize, window::Window};

pub struct GpuContext {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub is_surface_configured: bool,
    pub window: Arc<Window>,
    pub size: PhysicalSize<u32>
}

impl GpuContext {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::PRIMARY,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await?;

        let features = wgpu::Features::TEXTURE_BINDING_ARRAY
            | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
            | wgpu::Features::PARTIALLY_BOUND_BINDING_ARRAY;
            
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: None,
                required_features: features,
                required_limits: Default::default(),
                experimental_features: ExperimentalFeatures::disabled(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };


        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            size
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;

            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;

            self.size = PhysicalSize { width, height };
        }
    }

    pub fn get_output_view(&self) -> Result<(wgpu::SurfaceTexture, wgpu::TextureView), OutputViewError> {
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                self.surface.configure(&self.device, &self.config);
                surface_texture
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                return Err(OutputViewError::SkipFrame);
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Err(OutputViewError::SkipFrame);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                return Err(OutputViewError::Fatal);
            }
        };

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        Ok((output, view))
    } 
}

#[derive(Clone, Copy, Error, Debug)]
pub enum OutputViewError {
    #[error("Skip Frame")]
    SkipFrame,
    #[error("Fatal error in creating output view")]
    Fatal
}