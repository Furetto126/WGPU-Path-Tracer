use wgpu::util::DeviceExt;

use crate::{renderer::GpuContext, scene::{BVH, BvhNode, Camera, Model, model::PbrMaterial}};

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

    pub fn build_buffers(&mut self, ctx: &GpuContext) {
        let mut vertices = vec![];
        let mut uvs = vec![];
        let mut indices = vec![];
        let mut emissive_triangles = vec![];
        let mut triangle_materials = vec![];
        let mut materials = vec![];
        // TODO: This will only work for one model,
        //       a TLAS implementation must be created!
        let mut bvh_nodes = vec![];
        let mut bvh_triangle_indices = vec![];

        let mut vertex_offset = 0;
        let mut triangle_offset = 0;
        let mut material_offset = 0;
        for model in &self.models {
            let td = &model.triangles_data;

            indices.extend(td.indices.iter().map(|i| i + vertex_offset));

            emissive_triangles.extend(td.emissive_triangles.iter().map(|i| {
                if *i == !0 { !0 } else { i + triangle_offset }
            }));

            triangle_materials.extend(td.triangles_material.iter().map(|m| {
                if *m == !0 { !0 } else { m + material_offset }
            }));
            
            vertices.extend_from_slice(&td.vertices);
            uvs.extend_from_slice(&td.uvs);
            materials.extend_from_slice(&model.materials);
            
            let mut bvh = BVH::new(model);
            bvh.build();
            bvh_nodes.extend(bvh.nodes);
            bvh_triangle_indices.extend(bvh.triangle_indices);

            vertex_offset += td.vertices.len() as u32;
            triangle_offset += td.indices.len() as u32 / 3;
            material_offset += model.materials.len() as u32;
        }

        // If all models had no emissive triangle, then 
        // the emissive triangles count is 0.
        let mut emissive_triangles_count = emissive_triangles.len() as u32;
        for e in &emissive_triangles {
            if *e == !0 {
                emissive_triangles_count -= 1;
            }
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

        /*for t in emissive_triangles.iter() {
            let i = (t * 3) as usize;
            let (i0, i1, i2) = (indices[i] as usize, indices[i+1] as usize, indices[i+2] as usize);
            let (v1, v2, v3) = (vertices[i0], vertices[i1], vertices[i2]);
            let (uv1, uv2, uv3) = (uvs[i0], uvs[i1], uvs[i2]);
            let material = triangle_materials[*t as usize];

            println!(
                "TRIANGLE {t}: [{:.2?}, {:.2?}, {:.2?}] | UVs: [{:.2?}, {:.2?}, {:.2?}] | Mat: {}",
                v1, v2, v3, uv1, uv2, uv3, material
            );
        }*/

        self.buffers = SceneBuffers::new(
            ctx,
            false,
            vertices,
            uvs,
            indices,
            emissive_triangles_count,
            emissive_triangles,
            triangle_materials,
            materials,
            bvh_nodes,
            bvh_triangle_indices
        );
    }
}

#[derive(Debug, Clone)]
pub struct SceneBuffers {
    pub scene_empty_buffer: wgpu::Buffer,
    pub vertex_buffer: wgpu::Buffer,
    pub uv_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub emissive_triangles_count_buffer: wgpu::Buffer,
    pub emissive_triangles_buffer: wgpu::Buffer,
    pub material_index_buffer: wgpu::Buffer,
    pub material_buffer: wgpu::Buffer,
    pub bvh_nodes_buffer: wgpu::Buffer,
    pub bvh_triangle_indices_buffer: wgpu::Buffer,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl SceneBuffers {
    pub fn new(
        ctx: &GpuContext,
        is_scene_empty: bool,
        vertices: Vec<[f32; 4]>,
        uvs: Vec<[f32; 2]>,
        indices: Vec<u32>,
        emissive_triangles_count: u32,
        emissive_triangles: Vec<u32>,
        material_indices_per_triangle: Vec<u32>,
        materials: Vec<PbrMaterial>,
        bvh_nodes: Vec<BvhNode>,
        bvh_triangle_indices: Vec<u32>
    ) -> Self {
        let scene_empty_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Is Scene Empty Buffer"),
            contents: bytemuck::cast_slice(&[is_scene_empty as u32; 1]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
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
            label: Some("Indices Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let emissive_triangles_count_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Emissive Triangles Count Buffer"),
            contents: bytemuck::cast_slice(&[emissive_triangles_count; 1]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let emissive_triangles_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Emissive Indices Buffer"),
            contents: bytemuck::cast_slice(&emissive_triangles),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let material_index_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Indices Buffer"),
            contents: bytemuck::cast_slice(&material_indices_per_triangle),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let material_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Materials Buffer"),
            contents: bytemuck::cast_slice(&materials),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let bvh_nodes_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BVH Nodes Buffer"),
            contents: bytemuck::cast_slice(&bvh_nodes),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let bvh_triangle_indices_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BVH Triangle Indices Buffer"),
            contents: bytemuck::cast_slice(&bvh_triangle_indices),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let bind_group_layout = SceneBuffers::create_bind_group_layout(ctx);

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: scene_empty_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: vertex_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uv_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: index_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: emissive_triangles_count_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: emissive_triangles_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: material_index_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: material_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: bvh_nodes_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: bvh_triangle_indices_buffer.as_entire_binding()
                },
            ],
        });

        Self {
            scene_empty_buffer,
            vertex_buffer,
            uv_buffer,
            index_buffer,
            emissive_triangles_count_buffer,
            emissive_triangles_buffer,
            material_index_buffer,
            material_buffer,
            bvh_nodes_buffer,
            bvh_triangle_indices_buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn new_dummmy(ctx: &GpuContext) -> Self {
        Self::new(
            ctx,
            true,
            vec![[0.0; 4]; 3],  
            vec![[0.0; 2]; 3],  
            vec![0, 1, 2],
            0,
            vec![0, 1, 2],
            vec![0],
            vec![PbrMaterial::default()],
            vec![BvhNode::default()],
            vec![0, 1, 2]
        )
    } 

    pub fn create_bind_group_layout(ctx: &GpuContext) -> wgpu::BindGroupLayout {
        ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Scene Bind Group Layout"),
            entries: &[
                // Is scene empty
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None
                },
                // Vertices
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
                // UVs
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
                // Indices
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
                // Emissive triangles count
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None
                },
                // Emissive indices
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
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
                    binding: 6,
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
                    binding: 7,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None
                },
                // BVH Nodes
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None
                },
                // BVH Triangle Indices
                wgpu::BindGroupLayoutEntry {
                    binding: 9,
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