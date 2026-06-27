use std::{collections::{HashMap, HashSet}, ops::Div};

use anyhow::bail;
use cgmath::{Matrix4, Point2, Point3, SquareMatrix, Transform};
use wgpu::util::DeviceExt;

use crate::{renderer::GpuContext, texture::Texture};

pub struct Model {
    pub triangles_data: TrianglesData,
    pub materials: Vec<PbrMaterial>,
    pub textures: Vec<ModelTexture>
}

impl Model {
    pub fn load_static_model(ctx: &GpuContext, path: &str) -> anyhow::Result<Self> {
        let (document, buffers, images) = gltf::import(path)?;
        let materials = PbrMaterial::extract_materials(&document);

        let scene = document
            .default_scene()
            .unwrap_or_else(|| document.scenes().next().expect("gltf has no scenes"));

        let triangles_data = TrianglesData::load_from_scene(scene, &buffers, &materials);

        let unique_image_indices: Vec<u32> = materials.iter()
            .flat_map(|m| [m.base_color_texture, m.emissive_texture, m.metallic_roughness_texture, m.normal_texture])
            .filter(|&i| i != !0)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let remap: HashMap<u32, u32> = unique_image_indices.iter()
            .enumerate()
            .map(|(new_idx, &old_idx)| (old_idx, new_idx as u32))
            .collect();

        let mut textures = vec![];
        for &img_idx in &unique_image_indices {
            let ty = materials.iter().find_map(|m| {
                if m.base_color_texture == img_idx { Some(ModelTextureType::BaseColor) }
                else if m.emissive_texture == img_idx { Some(ModelTextureType::Emissive) }
                else if m.metallic_roughness_texture == img_idx { Some(ModelTextureType::MetallicRoughness) }
                else if m.normal_texture == img_idx { Some(ModelTextureType::Normal) }
                else { None }
            }).unwrap(); // TODO: Propagate error instead of panic.
            textures.push(ModelTexture::new(ctx, &images[img_idx as usize], ty)?);
        }

        let remap_idx = |i: u32| if i == !0 { !0 } else { remap[&i] };
        let materials: Vec<PbrMaterial> = materials.into_iter().map(|mut m| {
            m.base_color_texture = remap_idx(m.base_color_texture);
            m.emissive_texture = remap_idx(m.emissive_texture);
            m.metallic_roughness_texture = remap_idx(m.metallic_roughness_texture);
            m.normal_texture = remap_idx(m.normal_texture);
            m
        })
        .collect();

        Ok(Self { triangles_data, materials, textures })
    }
}

pub enum ModelTextureType {
    BaseColor, Emissive, MetallicRoughness, Normal
}

#[derive(Debug, Clone)]
pub struct ModelTexture {
    pub texture: Texture,
}

impl ModelTexture {
    pub fn new(ctx: &GpuContext, image: &gltf::image::Data, ty: ModelTextureType) -> anyhow::Result<Self> {
        let rgba: Vec<u8> = match image.format {
            gltf::image::Format::R8G8B8 => {
                image.pixels.chunks_exact(3)
                    .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
                    .collect()
            }
            gltf::image::Format::R8G8B8A8 => image.pixels.clone(),
            gltf::image::Format::R8 => image.pixels.iter().flat_map(|&r| [r, 0, 0, 255]).collect(),
            _ => bail!("Unsupported image format {:?}", image.format)
        };

        let texture_format = match ty {
            ModelTextureType::BaseColor | ModelTextureType::Emissive => wgpu::TextureFormat::Rgba8UnormSrgb,
            ModelTextureType::MetallicRoughness | ModelTextureType::Normal => wgpu::TextureFormat::Rgba8Unorm
        };

        let size = wgpu::Extent3d {
            width: image.width,
            height: image.height,
            depth_or_array_layers: 1,
        };
        let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;

        Ok(Self { texture: Texture::new_with_data(ctx, size, usage, texture_format, &rgba) })
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct PbrMaterial {
    base_color_factor: [f32; 4],
    pub base_color_texture: u32,

    metallic_factor: f32,
    roughness_factor: f32,
    pub metallic_roughness_texture: u32,

    emissive_factor: [f32; 3],
    pub emissive_texture: u32,

    pub normal_texture: u32,
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
                    base_color_texture: pbr.base_color_texture()
                        .map(|i| i.texture().source().index() as u32)
                        .unwrap_or(!0),
                    metallic_factor: pbr.metallic_factor(),
                    roughness_factor: pbr.roughness_factor(),
                    metallic_roughness_texture: pbr.metallic_roughness_texture()
                        .map(|i| i.texture().source().index() as u32)
                        .unwrap_or(!0),
                    emissive_factor: mat.emissive_factor(),
                    emissive_texture: mat.emissive_texture()
                        .map(|i| i.texture().source().index() as u32)
                        .unwrap_or(!0),
                    normal_texture: mat.normal_texture()
                        .map(|i| i.texture().source().index() as u32)
                        .unwrap_or(!0),
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