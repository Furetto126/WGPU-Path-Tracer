use wgpu::util::DeviceExt;

use crate::renderer::{GpuContext, sobol_generator};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
pub struct RendererUniforms {
    pub frame: u32,
}

#[derive(Debug, Clone)]
pub struct RendererUniformsBundle {
    pub value: RendererUniforms,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,

    renderer_uniforms_buffer: wgpu::Buffer,
    sobol_buffer: wgpu::Buffer,
}

impl RendererUniformsBundle {
    pub fn new(ctx: &GpuContext, value: RendererUniforms) -> Self {
        let renderer_uniforms_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Renderer Uniforms Buffer"),
            contents: bytemuck::cast_slice(&[value]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sobol_buffer = sobol_generator::generate_sobol_directions_buffer(ctx);

        let bind_group_layout = ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Renderer Uniforms Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ]
        });

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Renderer Uniforms Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: renderer_uniforms_buffer.as_entire_binding() 
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sobol_buffer.as_entire_binding(),
                },            
            ],
        });

        Self { 
            value,
            bind_group_layout,
            bind_group,
            renderer_uniforms_buffer,
            sobol_buffer,
        }
    }

    pub fn update_unforms(&mut self, ctx: &GpuContext, new_val: RendererUniforms) {
        if self.value != new_val {
            self.value = new_val;
            ctx.queue.write_buffer(
                &self.renderer_uniforms_buffer, 0, bytemuck::cast_slice(&[self.value]));
        }
    }
}