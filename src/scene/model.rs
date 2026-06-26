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

        let triangles_data = TrianglesData::load_from_scene(scene, &buffers, &materials);

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
        let mut materials: Vec<PbrMaterial> = 
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
            .collect();

        if materials.is_empty() {
            materials.push(PbrMaterial::default());
        }

        materials
    }
}

#[derive(Debug, Clone)]
pub struct TrianglesData {
    pub vertices: Vec<[f32; 4]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub emissive_triangles: Vec<u32>,

    pub triangles_material: Vec<u32>
}

impl TrianglesData {
    pub fn load_from_scene(scene: gltf::Scene, buffers: &[gltf::buffer::Data], pbr_materials: &Vec<PbrMaterial>) -> TrianglesData {
        let mut triangles_data = TrianglesData { 
            vertices: vec![],
            uvs: vec![],
            indices: vec![],
            emissive_triangles: vec![],
            triangles_material: vec![]
        };

        for node in scene.nodes() {
            Self::walk_node(&node, Matrix4::identity(), buffers, pbr_materials, &mut triangles_data);
        }

        // Fill with dummy / unimportant data if not present
        if triangles_data.uvs.len() < triangles_data.vertices.len() {
            triangles_data.uvs.resize(triangles_data.vertices.len(), [0.0; 2]);
        }

        if triangles_data.indices.is_empty() {
            triangles_data.indices = (0..triangles_data.vertices.len() as u32).collect();
        }

        if triangles_data.emissive_triangles.is_empty() {
            triangles_data.emissive_triangles.push(!0);
        }

        if triangles_data.triangles_material.len() < triangles_data.indices.len().div(3) {
            triangles_data.triangles_material.resize(triangles_data.indices.len().div(3), !0);
        }

        triangles_data
    }

    fn walk_node(
        node: &gltf::Node,
        parent_world: Matrix4<f32>,
        buffers: &[gltf::buffer::Data],
        pbr_materials: &Vec<PbrMaterial>,
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
                    .map(|i| i.into_u32().map(|idx| idx + triangles_data.vertices.len() as u32).collect())
                    .unwrap_or_default();

                let material_index = prim.material().index().map(|i| i as u32).unwrap_or(!0);

                let base_triangle = (triangles_data.indices.len() / 3) as u32;
                let triangle_count = (indices.len() / 3) as u32;

                triangles_data.vertices.extend_from_slice(&transformed_vertices);
                triangles_data.uvs.extend_from_slice(&uvs);
                triangles_data.indices.extend(indices.iter());

                let em_f = pbr_materials[material_index as usize].emissive_factor;
                if em_f[0] != 0.0 || em_f[1] != 0.0 || em_f[2] != 0.0 || pbr_materials[material_index as usize].emissive_texture != !0 {
                    for tri_offset in 0..triangle_count {
                        triangles_data.emissive_triangles.push(base_triangle + tri_offset);
                    }
                }

                triangles_data.triangles_material.extend(
                    std::iter::repeat_n(material_index, indices.len().div(3))
                );
            }
        }

        for child in node.children() {
            Self::walk_node(&child, world, buffers, pbr_materials, triangles_data);
        }
    }
}