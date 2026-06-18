use std::sync::OnceLock;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState, ColorTargetState, ColorWrites, ComputePipeline, ComputePipelineDescriptor, Extent3d, FragmentState, MultisampleState, PipelineCompilationOptions, PrimitiveState, RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerDescriptor, ShaderModule, ShaderStages, TextureFormat::Rgba32Float, TextureUsages, VertexState, util::DeviceExt
};

use crate::{renderer::GpuContext, scene::Camera, texture::Texture};

#[derive(Debug, Clone)]
struct PathTracerBindGroups {
    // Compute bind groups
    // -------------------
    // @group(0)
    compute_bind_group: BindGroup,
    compute_bind_group_layout: BindGroupLayout,

    // Display bind groups
    // -------------------
    // @group(0)
    display_bind_group: BindGroup,
    display_bind_group_layout: BindGroupLayout,
}

#[derive(Debug, Clone)]
pub struct PathTracer {
    compute_pipeline: ComputePipeline,
    display_pipeline: RenderPipeline,

    bind_groups: PathTracerBindGroups,

    result_texture: Texture,
    texture_sampler: Sampler,
}

impl PathTracer {
    pub fn new(ctx: &GpuContext, compute_shader: ShaderModule, display_shader: ShaderModule, camera: &Camera) -> Self {
        // Shared Output/Input Texture
        // ---------------------------
        let texture_sampler = ctx.device.create_sampler(&SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let result_texture = Texture::new(
            ctx,
            Extent3d {
                width: ctx.size.width,
                height: ctx.size.height,
                depth_or_array_layers: 1,
            },
            TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING
        );

        let bind_groups = Self::generate_texture_bind_group(ctx, &result_texture, &texture_sampler);

        // Compute Shader setup
        // --------------------
        let compute_pipeline_layout = ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[
                Some(&bind_groups.compute_bind_group_layout),
                Some(&camera.camera_bind_group_layout)
            ],
            ..Default::default()
        });
        let compute_pipeline = ctx.device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: None,
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        // Display Pipeline
        // Composed of a Vertex + Fragment shader
        // --------------------------------------
        let display_pipeline_layout =
            ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[Some(&bind_groups.display_bind_group_layout)],
                immediate_size: 0
            });

        let display_pipeline = ctx.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Display Pipeline"),
            layout: Some(&display_pipeline_layout),
            vertex: VertexState {
                module: &display_shader,
                entry_point: None,
                buffers: &[Vertex::desc()],
                compilation_options: PipelineCompilationOptions::default()
            },
            fragment: Some(FragmentState {
                module: &display_shader,
                entry_point: None,
                targets: &[Some(ColorTargetState {
                    format: ctx.config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false
            },
            multiview_mask: None,
            cache: None,
        });

        let _ = FULLSCREEN_VERTEX_BUFFER.set(ctx.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(QUAD_VERTICES),
                usage: wgpu::BufferUsages::VERTEX
            }
        ));

        let _ = FULLSCREEN_INDEX_BUFFER.set(ctx.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(QUAD_INDICES),
                usage: wgpu::BufferUsages::INDEX,
            }
        ));

        Self {
            compute_pipeline,
            display_pipeline,
            bind_groups,
            result_texture,
            texture_sampler,
        }
    }

    pub fn render(&mut self, ctx: &GpuContext, camera: &Camera) -> anyhow::Result<()> {
        let old_size = self.result_texture.get_size();
        let new_size = Extent3d {
            width: ctx.size.width,
            height: ctx.size.height,
            depth_or_array_layers: 1,
        };

        if old_size != new_size {
            self.result_texture.resize(
                ctx, new_size
            );

            self.bind_groups = Self::generate_texture_bind_group(ctx, &self.result_texture, &self.texture_sampler);
        } 

        // Get SurfaceTexture and TextureView to render on.
        let (output, view) = match ctx.get_output_view() {
            Ok(ov) => ov,
            Err(crate::renderer::gpu_context::OutputViewError::SkipFrame) => return Ok(()),
            Err(crate::renderer::gpu_context::OutputViewError::Fatal) => anyhow::bail!("Lost device")
        };

        let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder")
        });

        // Compute shader Path Tracing pass
        // --------------------------------
        {
            let mut path_tracing_pass = encoder.begin_compute_pass(&Default::default());
            path_tracing_pass.set_pipeline(&self.compute_pipeline);

            // Set bind groups needed in rendering by the Compute Shader.
            path_tracing_pass.set_bind_group(0, &self.bind_groups.compute_bind_group, &[]);
            path_tracing_pass.set_bind_group(1, &camera.camera_bind_group, &[]);

            let wg_x = self.result_texture.get_size().width.div_ceil(8);
            let wg_y = self.result_texture.get_size().height.div_ceil(8); 

            // Execute Compute Shader.
            path_tracing_pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        // Vertex + Fragment display pass
        // ------------------------------
        {
            let mut display_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Display Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            display_pass.set_pipeline(&self.display_pipeline);
            display_pass.set_bind_group(0, &self.bind_groups.display_bind_group, &[]);
            display_pass.set_vertex_buffer(0, FULLSCREEN_VERTEX_BUFFER.get().unwrap().slice(..));
            display_pass.set_index_buffer(FULLSCREEN_INDEX_BUFFER.get().unwrap().slice(..), wgpu::IndexFormat::Uint16);
            display_pass.draw_indexed(0..QUAD_INDICES.len() as u32, 0, 0..1);
        }

        ctx.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn generate_texture_bind_group(ctx: &GpuContext, texture: &Texture, texture_sampler: &Sampler) -> PathTracerBindGroups {
        let compute_bind_group_layout = ctx.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Compute Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None
                },
            ]
        });

        let display_bind_group_layout = ctx.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Display Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                }
            ],
        });

        let compute_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Result Texture Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(texture.get_view()),
                }
            ],
        });

        let display_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Result Texture Bind Group"),
            layout: &display_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(texture.get_view())
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(texture_sampler)
                }
            ],
        });

        PathTracerBindGroups {
            compute_bind_group,
            compute_bind_group_layout,
            display_bind_group,
            display_bind_group_layout,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// Full-screen Quad
// ----------------
const QUAD_VERTICES: &[Vertex] = &[
    Vertex { position: [-1.0, -1.0], uv: [0.0, 1.0] },
    Vertex { position: [ 1.0, -1.0], uv: [1.0, 1.0] },
    Vertex { position: [-1.0,  1.0], uv: [0.0, 0.0] },
    Vertex { position: [ 1.0,  1.0], uv: [1.0, 0.0] },
];

const QUAD_INDICES: &[u16] = &[0, 1, 2, 2, 1, 3];

static FULLSCREEN_VERTEX_BUFFER: OnceLock<wgpu::Buffer> = OnceLock::new();
static FULLSCREEN_INDEX_BUFFER: OnceLock<wgpu::Buffer> = OnceLock::new();