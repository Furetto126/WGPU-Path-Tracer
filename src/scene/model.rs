use std::ops::Div;

use cgmath::{Matrix4, Point2, Point3, SquareMatrix, Transform};

pub struct Model {
    pub triangles_data: TrianglesData,
    pub materials: Vec<PbrMaterial>
}

impl Model {
    pub fn load_static_scene(path: &str) -> anyhow::Result<Self> {
        let (document, buffers, _images) = gltf::import(path)?;
        let materials = PbrMaterial::extract_materials(&document);

        let scene = document
            .default_scene()
            .unwrap_or_else(|| document.scenes().next().expect("gltf has no scenes"));

        let triangles_data = TrianglesData::load_from_scene(scene, &buffers);

        Ok(Self { triangles_data, materials })
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct PbrMaterial {
    base_color_factor: [f32; 4],
    base_color_texture: u32,

    metallic_factor: f32,
    roughness_factor: f32,
    metallic_roughness_texture: u32,

    emissive_factor: [f32; 3],
    emissive_texture: u32,

    normal_texture: u32,
    _pad: [f32; 3]
}

impl PbrMaterial {
    pub fn extract_materials(document: &gltf::Document) -> Vec<PbrMaterial> {
        document
            .materials()
            .map(|mat| {
                let pbr = mat.pbr_metallic_roughness();
                PbrMaterial {
                    base_color_factor: pbr.base_color_factor(),
                    base_color_texture: pbr.base_color_texture().map(|i| i.texture().index() as u32).unwrap_or(!0),
                    metallic_factor: pbr.metallic_factor(),
                    roughness_factor: pbr.roughness_factor(),
                    metallic_roughness_texture: pbr.metallic_roughness_texture().map(|i| i.texture().index() as u32).unwrap_or(!0),
                    emissive_factor: mat.emissive_factor(),
                    emissive_texture: mat.emissive_texture().map(|i| i.texture().index() as u32).unwrap_or(!0),
                    normal_texture: mat.normal_texture().map(|i| i.texture().index() as u32).unwrap_or(!0),
                    _pad: [0.0; 3]
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct TrianglesData {
    pub vertices: Vec<[f32; 4]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,

    pub triangles_material: Vec<u32>
}

impl TrianglesData {
    pub fn load_from_scene(scene: gltf::Scene, buffers: &[gltf::buffer::Data]) -> TrianglesData {
        let mut triangles_data = TrianglesData { 
            vertices: vec![],
            uvs: vec![],
            indices: vec![],
            triangles_material: vec![]
        };

        for node in scene.nodes() {
            Self::walk_node(&node, Matrix4::identity(), buffers, &mut triangles_data);
        }

        triangles_data
    }

    fn walk_node(
        node: &gltf::Node,
        parent_world: Matrix4<f32>,
        buffers: &[gltf::buffer::Data],
        triangles_data: &mut TrianglesData,
    ) {
        let local: Matrix4<f32> = node.transform().matrix().into();
        let world = parent_world * local;

        if let Some(mesh) = node.mesh() {
            for prim in mesh.primitives() {
                let reader = prim.reader(|b| Some(&buffers[b.index()]));

                let transformed_vertices: Vec<[f32; 4]> = reader
                    .read_positions()
                    .map(|it| 
                        it.map(Point3::from)
                        .map(|v| world.transform_point(v))
                        .map(|v| [v.x, v.y, v.z, 0.0])
                        .collect())
                    .unwrap_or_default();

                let uvs: Vec<[f32; 2]> = reader
                    .read_tex_coords(0)
                    .map(|tc| tc.into_f32().map(Point2::from).map(|v| v.into()).collect())
                    .unwrap_or_default();

                let indices: Vec<u32> = reader
                    .read_indices()
                    .map(|i| i.into_u32().collect())
                    .unwrap_or_default();

                let material_index = prim.material().index().map(|i| i as u32).unwrap_or(!0);

                let index_offset = triangles_data.vertices.len() as u32;
                triangles_data.vertices.extend_from_slice(&transformed_vertices);
                triangles_data.uvs.extend_from_slice(&uvs);
                triangles_data.indices.extend(indices.iter().map(|i| i + index_offset));
                triangles_data.triangles_material.extend(
                    std::iter::repeat_n(material_index, indices.len().div(3))
                );
            }
        }

        for child in node.children() {
            Self::walk_node(&child, world, buffers, triangles_data);
        }
    }
}