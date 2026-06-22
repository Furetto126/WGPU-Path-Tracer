use cgmath::{InnerSpace, Matrix, Matrix4, Point3, SquareMatrix, Transform, Vector2, Vector3};

#[derive(Debug, Clone)]
pub struct PbrMaterial {
    base_color_factor: [f32; 4],
    base_color_texture: u32,

    metallic_factor: f32,
    roughness_factor: f32,
    metallic_roughness_texture: u32,

    emissive_factor: [f32; 3],
    emissive_texture: u32,

    normal_texture: u32,
}

#[derive(Debug, Clone)]
pub struct Triangle {
    positions: [Point3<f32>; 3],
    normals: [Vector3<f32>; 3],
    uvs: [Vector2<f32>; 3],
    material_index: u32
}

fn extract_materials(document: &gltf::Document) -> Vec<PbrMaterial> {
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
            }
        })
        .collect()
}

fn walk_node(
    node: &gltf::Node,
    parent_world: Matrix4<f32>,
    buffers: &[gltf::buffer::Data],
    triangles: &mut Vec<Triangle>
) {
    let local: Matrix4<f32> = node.transform().matrix().into();
    let world = parent_world * local;

    if let Some(mesh) = node.mesh() {
        let normal_mat = world.invert().unwrap().transpose();

        for prim in mesh.primitives() {
            let reader = prim.reader(|b| Some(&buffers[b.index()]));

            let positions: Vec<Point3<f32>> = reader
                .read_positions()
                .map(|it| it.map(Point3::from).collect())
                .unwrap_or_default();

            let normals: Vec<Vector3<f32>> = reader
                .read_normals()
                .map(|it| it.map(Vector3::from).collect())
                .unwrap_or_default();

            let uvs: Vec<Vector2<f32>> = reader
                .read_tex_coords(0)
                .map(|tc| tc.into_f32().map(Vector2::from).collect())
                .unwrap_or_default();

            let indices: Vec<u32> = reader
                .read_indices()
                .map(|i| i.into_u32().collect())
                .unwrap_or_default();

            let material_index = prim.material().index().map(|i| i as u32).unwrap_or(!0);

            // Baking matrix into triangles
            for tri in indices.chunks_exact(3) {
                let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);

                let positions = [
                    world.transform_point(positions[i0]),
                    world.transform_point(positions[i1]),
                    world.transform_point(positions[i2]),
                ];
                let normals = [
                    normal_mat.transform_vector(normals[i0]).normalize(),
                    normal_mat.transform_vector(normals[i1]).normalize(),
                    normal_mat.transform_vector(normals[i2]).normalize(),
                ];
                let uvs = [
                    uvs.get(i0).copied().unwrap_or([0.0, 0.0].into()),
                    uvs.get(i1).copied().unwrap_or([0.0, 0.0].into()),
                    uvs.get(i2).copied().unwrap_or([0.0, 0.0].into()),
                ];

                triangles.push(Triangle { positions, normals, uvs, material_index });

            }
        }
    }

    for child in node.children() {
        walk_node(&child, world, buffers, triangles);
    }
}

pub fn load_static_scene(path: &str) -> anyhow::Result<(Vec<PbrMaterial>, Vec<Triangle>)> {
    let (document, buffers, _images) = gltf::import(path)?;
    let materials = extract_materials(&document);

    let mut triangles = vec![];
    let scene = document
        .default_scene()
        .unwrap_or_else(|| document.scenes().next().expect("gltf has no scenes"));

    for node in scene.nodes() {
        walk_node(&node, Matrix4::identity(), &buffers, &mut triangles);
    }

    Ok((materials, triangles))
}