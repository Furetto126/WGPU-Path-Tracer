use std::sync::Arc;

use wesl::include_wesl;
use wgpu::include_wgsl;
use winit::{event_loop::ActiveEventLoop, keyboard::KeyCode, window::Window};

use crate::{renderer::{GpuContext, PathTracer, sobol_generator::SobolDirectionsBindGroup}, scene::Camera};

pub struct RendererState {
    gpu_context: GpuContext,
    camera: Camera,
    path_tracer: PathTracer,
    sobol_dirs: SobolDirectionsBindGroup,
}

impl RendererState {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let gpu_context = GpuContext::new(window).await?;
        let camera = Camera::new(
            &gpu_context,
            (0.0, 0.0, 0.0).into(),
            (0.0, 0.0, -1.0).into(),
            45.0,
            gpu_context.size.width as f32 / gpu_context.size.height as f32 
        );


        let compute_shader = gpu_context.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_wesl!("path_tracer").into()),
        });
        let display_shader = gpu_context.device.create_shader_module(include_wgsl!("../shaders/display.wgsl"));

        let sobol_dirs = SobolDirectionsBindGroup::generate_sobol_directions_buffer(&gpu_context);

        let path_tracer = PathTracer::new(
            &gpu_context,
            compute_shader,
            display_shader,
            vec![
                &camera.camera_bind_group_layout,
                &sobol_dirs.bind_group_layout
            ]
        );

        Ok(Self {
            gpu_context,
            camera,    
            path_tracer,
            sobol_dirs
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu_context.resize(width, height);
    }

    pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => { let _ = self.camera.camera_controller.handle_key(code, is_pressed); }
        };
    }
    
    pub fn update(&mut self) {
        self.camera.update(&self.gpu_context);
    } 

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.gpu_context.window.request_redraw();

        if !self.gpu_context.is_surface_configured {
            return Ok(());
        }
            
        // RENDER!
        self.path_tracer.render(
            &self.gpu_context,
            vec![
                &self.camera.camera_bind_group,
                &self.sobol_dirs.bind_group
            ])
    }
}