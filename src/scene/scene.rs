use wgpu::util::DeviceExt;

use crate::{renderer::GpuContext, scene::{Camera, Model, model::PbrMaterial}};

pub struct Scene {
    pub camera: Camera,
    pub models: Vec<Model>,
    pub buffers: SceneBuffers
}

impl Scene {
    pub fn new(ctx: &GpuContext, camera: Camera) -> Self {
        Self { camera, models: vec![], buffers: SceneBuffers::new_dummmy(ctx) }
    }

    pub fn load_model(&mut self, ctx: &GpuContext, path: &str) -> anyhow::Result<()> {
        let model = Model::load_static_scene(path)?;
        self.models.push(model);
        self.build_buffers(ctx);
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        return self.models.is_empty()
    }

    pub fn build_buffers(&mut self, ctx: &GpuContext) {
        let mut vertices = vec![];
        let mut uvs = vec![];
        let mut indices = vec![];
        let mut triangle_materials = vec![];
        let mut materials = vec![];

        let mut vertex_offset = 0;
        let mut material_offset = 0;
        for model in &self.models {
            let td = &model.triangles_data;

            indices.extend(td.indices.iter().map(|i| i + vertex_offset));
            triangle_materials.extend(td.triangles_material.iter().map(|m| {
                if *m == !0 { !0 } else { m + material_offset }
            }));
            
            vertices.extend_from_slice(&td.vertices);
            uvs.extend_from_slice(&td.uvs);
            materials.extend_from_slice(&model.materials);

            vertex_offset += td.vertices.len() as u32;
            material_offset += model.materials.len() as u32;
        }

        // Models might have missing UVs, Indices etc. 
        if uvs.len() < vertices.len() {
            uvs.resize(vertices.len(), [0.0; 2]);
        }

        if indices.is_empty() {
            indices = (0..vertices.len() as u32).collect();
        }

        if triangle_materials.len() < indices.len() / 3 {
            triangle_materials.resize(indices.len() / 3, !0);
        }

        if materials.is_empty() {
            materials.push(PbrMaterial::default());
        }

        // DEBUG
        /*for (i, tri_idx) in indices.chunks_exact(3).enumerate() {
            let (v1, v2, v3) = (vertices[tri_idx[0] as usize], vertices[tri_idx[1] as usize], vertices[tri_idx[2] as usize]);
            let (uv1, uv2, uv3) = (uvs[tri_idx[0] as usize], uvs[tri_idx[1] as usize], uvs[tri_idx[2] as usize]);
            let material = triangle_materials[i];

            println!(
                "TRIANGLE {i}: [{:.2?}, {:.2?}, {:.2?}] | UVs: [{:.2?}, {:.2?}, {:.2?}] | Mat: {}",
                v1, v2, v3, uv1, uv2, uv3, material
            );
        }*/

        self.buffers = SceneBuffers::new(
            ctx,
            vertices,
            uvs,
            indices,
            triangle_materials,
            materials
        );
    }
}

#[derive(Debug, Clone)]
pub struct SceneBuffers {
    pub vertex_buffer: wgpu::Buffer,
    pub uv_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub material_index_buffer: wgpu::Buffer,
    pub material_buffer: wgpu::Buffer,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl SceneBuffers {
    pub fn new(
        ctx: &GpuContext,
        vertices: Vec<[f32; 4]>,
        uvs: Vec<[f32; 2]>,
        indices: Vec<u32>,
        material_indices_per_triangle: Vec<u32>,
        materials: Vec<PbrMaterial>
    ) -> Self {
        let vertex_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let uv_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UV Buffer"),
            contents: bytemuck::cast_slice(&uvs),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let index_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let material_index_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("materials"),
            contents: bytemuck::cast_slice(&material_indices_per_triangle),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let material_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("materials"),
            contents: bytemuck::cast_slice(&materials),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let bind_group_layout = SceneBuffers::create_bind_group_layout(ctx);

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertex_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uv_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: index_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: material_index_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: material_buffer.as_entire_binding()
                },
            ],
        });

        Self {
            vertex_buffer,
            uv_buffer,
            index_buffer,
            material_index_buffer,
            material_buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn new_dummmy(ctx: &GpuContext) -> Self {
        Self::new(
            ctx,
            vec![[0.0; 4]; 3],  
            vec![[0.0; 2]; 3],  
            vec![0, 1, 2],
            vec![0],
            vec![PbrMaterial::default()]
        )
    } 

    pub fn create_bind_group_layout(ctx: &GpuContext) -> wgpu::BindGroupLayout {
        ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Scene Bind Group Layout"),
            entries: &[
                // Vertices
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None
                },
                // UVs
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None
                },
                // Indices
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None
                },
                // Material Indices
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None
                },
                // Materials
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None
                }
            ],
        })
    }
}