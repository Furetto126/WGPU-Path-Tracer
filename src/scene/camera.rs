use cgmath::{Deg, Matrix4, Point3, SquareMatrix, Vector3, perspective};
use wgpu::{BindGroup, BindGroupLayout, Buffer, util::DeviceExt};
use winit::keyboard::KeyCode;

use crate::renderer::GpuContext;

#[derive(Debug, Clone, Copy)]
pub struct CameraTransform {
    pub position: Point3<f32>,
    pub front: Vector3<f32>,
    pub up: Vector3<f32>,
    pub right: Vector3<f32>
}

impl CameraTransform {
    pub fn new(position: Point3<f32>, front: Vector3<f32>) -> Self {
        let up = Vector3::unit_y();
        let right = front.cross(up);
        Self {
            position,
            front,
            up,
            right,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CameraOptions {
    pub aspect_ratio: f32,
    pub fov: f32,
    pub znear: f32,
    pub zfar: f32,
}

#[derive(Debug, Clone)]
pub struct Camera {
    pub transform: CameraTransform,
    pub options: CameraOptions,
    pub camera_controller: CameraController,
    
    pub camera_uniform: CameraUniform,
    pub camera_buffer: Buffer,

    pub camera_bind_group_layout: BindGroupLayout, 
    pub camera_bind_group: BindGroup,
}

impl Camera {
    pub fn new(ctx: &GpuContext, position: Point3<f32>, front: Vector3<f32>, fov: f32, aspect_ratio: f32) -> Self {
        let transform = CameraTransform::new(position, front);
        let options = CameraOptions {
            aspect_ratio,
            fov,
            znear: 0.1,
            zfar: 100.0,
        };
        
        let camera_controller = CameraController::new(0.2);
        let camera_uniform = CameraUniform::new(&transform, &options);

        let camera_buffer = ctx.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let camera_bind_group_layout = ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });

        let camera_bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });

        Self {
            transform,
            options,
            camera_controller,
            camera_uniform,
            camera_buffer,
            camera_bind_group_layout,
            camera_bind_group,
        }
    }

    pub fn update(&mut self, ctx: &GpuContext) {
        //if self.camera_controller.update_transform(&mut self.transform) {
            self.camera_uniform.update(&self.transform, &self.options);

            ctx.queue.write_buffer(
                &self.camera_buffer,
                0,
                bytemuck::cast_slice(&[self.camera_uniform]));
        //}
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    inv_view_proj: [[f32; 4]; 4],
    cam_pos: [f32; 3],
    pub _pad: f32,
}

impl CameraUniform {
    pub fn new(transform: &CameraTransform, options: &CameraOptions) -> Self {
        Self {
            inv_view_proj: Self::generate_inv_view_proj(transform, options),
            cam_pos: transform.position.into(),
            _pad: 0.0
        }
    }

    pub fn update(&mut self, transform: &CameraTransform, options: &CameraOptions) {
        self.inv_view_proj = Self::generate_inv_view_proj(transform, options);
        self.cam_pos = transform.position.into();
    }

    fn generate_inv_view_proj(transform: &CameraTransform, options: &CameraOptions) -> [[f32; 4]; 4] {
        let view = Matrix4::look_to_rh(transform.position, transform.front, transform.up);
        let projection = perspective(Deg(options.fov), options.aspect_ratio, options.znear, options.zfar);
        let view_proj = Self::OPENGL_TO_WGPU_MATRIX * projection * view;

        if !view_proj.is_invertible() {
            return cgmath::Matrix4::identity().into();
        }

        return view_proj.invert().unwrap().into();
    }

    const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
        cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
        cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
        cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
        cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
    );
}

#[derive(Debug, Clone, Copy)]
pub struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_right_pressed: bool,
    is_left_pressed: bool,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    pub fn handle_key(&mut self, code: KeyCode, is_pressed: bool) -> bool {
        match code {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.is_forward_pressed = is_pressed;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.is_left_pressed = is_pressed;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.is_backward_pressed = is_pressed;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.is_right_pressed = is_pressed;
                true
            }
            _ => false,
        }
    }

    pub fn update_transform(&mut self, transform: &mut CameraTransform) -> bool {
        let mut should_update = false;
        if self.is_forward_pressed {
            transform.position += transform.front * self.speed;
            should_update = true;
        }
        if self.is_backward_pressed {
            transform.position -= transform.front * self.speed;
            should_update = true;
        }

        if self.is_right_pressed {
            transform.position += transform.right * self.speed;
            should_update = true;
        }
        if self.is_left_pressed {
            transform.position -= transform.right * self.speed;
            should_update = true;
        }

        self.is_forward_pressed = false;
        self.is_backward_pressed = false;
        self.is_right_pressed = false;
        self.is_left_pressed = false;

        should_update
    }
}