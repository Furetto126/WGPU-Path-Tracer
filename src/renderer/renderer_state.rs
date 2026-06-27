use std::sync::Arc;

use wesl::include_wesl;
use wgpu::include_wgsl;
use winit::{event_loop::ActiveEventLoop, keyboard::KeyCode, window::Window};

use crate::{renderer::{GpuContext, PathTracer, renderer_uniforms::{RendererUniforms, RendererUniformsBundle}}, scene::{Camera, Scene, SceneBuffers}};

pub struct RendererState {
    gpu_context: GpuContext,
    path_tracer: PathTracer,
    scene: Scene,
    renderer_uniforms: RendererUniformsBundle,

    timer: f32,
    frames: u32,
    fps: f32
}

impl RendererState {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let gpu_context = GpuContext::new(window).await?;
        let camera = Camera::new(
            &gpu_context,
            (0.0, 0.0, 5.0).into(),
            (0.0, 0.0, -1.0).into(),
            45.0,
            gpu_context.size.width as f32 / gpu_context.size.height as f32 
        );

        let mut scene = Scene::new(&gpu_context, camera);
        scene.load_model(&gpu_context, "assets/CarConcept.glb")?;

        let compute_shader = gpu_context.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_wesl!("path_tracer").into()),
        });
        let display_shader = gpu_context.device.create_shader_module(include_wgsl!("../shaders/display.wgsl"));

        let renderer_uniforms = RendererUniformsBundle::new(
            &gpu_context, 
            RendererUniforms {
                frame: 0,
            }
        );

        let path_tracer = PathTracer::new(
            &gpu_context,
            compute_shader,
            display_shader,
            vec![
                &scene.camera.camera_bind_group_layout,
                &renderer_uniforms.bind_group_layout,
                &scene.buffers.bind_group_layout,
            ]
        );

        Ok(Self {
            gpu_context,
            path_tracer,
            scene,    
            renderer_uniforms,
            timer: 0.0,
            frames: 0,
            fps: 0.0
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu_context.resize(width, height);
        self.path_tracer.resize_canvas_if_changed(&self.gpu_context);
    }

    pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => { let _ = self.scene.camera.camera_controller.handle_key(code, is_pressed); }
        };
    }
    
    pub fn update(&mut self) {
        self.scene.camera.update(&self.gpu_context);

        let mut new_val = self.renderer_uniforms.value;
        new_val.frame += 1;
        self.renderer_uniforms.update_unforms(&self.gpu_context, new_val);
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
                &self.scene.camera.camera_bind_group,
                &self.renderer_uniforms.bind_group,
                &self.scene.buffers.bind_group,
            ]
        )
    }
}