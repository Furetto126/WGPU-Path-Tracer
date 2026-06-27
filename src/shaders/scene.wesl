import package::intersection::Hit;

@group(3) @binding(0) var<storage, read> is_scene_empty: u32;
@group(3) @binding(1) var<storage, read> vertices: array<vec3<f32>>;
@group(3) @binding(2) var<storage, read> uvs: array<vec2<f32>>;
@group(3) @binding(3) var<storage, read> indices: array<u32>;
@group(3) @binding(4) var<storage, read> emissive_triangles_count: u32;
@group(3) @binding(5) var<storage, read> emissive_triangles: array<u32>;
@group(3) @binding(6) var<storage, read> triangle_materials: array<u32>;
@group(3) @binding(7) var<storage, read> materials: array<PbrMaterial>;
@group(3) @binding(8) var textures: binding_array<texture_2d<f32>>; // maybe array
@group(3) @binding(9) var textures_sampler: sampler;
@group(3) @binding(10) var<storage, read> bvh_nodes: array<BvhNode>;
@group(3) @binding(11) var<storage, read> bvh_triangle_indices: array<u32>;

struct PbrMaterial {
    base_color_factor: vec4f,
    base_color_texture: u32,

    metallic_factor: f32,
    roughness_factor: f32,
    metallic_roughness_texture: u32,

    emissive_factor: vec3f,
    emissive_texture: u32,

    normal_texture: u32
}

struct Sphere {
    center: vec3f,
    radius: f32,
    material: u32,
}

struct Triangle {
    v0: vec3f,
    v1: vec3f,
    v2: vec3f,
    
    material: u32
}

struct BvhNode {
    aabb_min: vec3f,
    left_child_or_first_triangle: u32,
    aabb_max: vec3f,
    num_triangles: u32
}

fn get_normal_world_space(hit: Hit) -> vec3f {
    let base_idx = hit.triangle_index * 3;
    let v0 = vertices[indices[base_idx]];
    let v1 = vertices[indices[base_idx+1]];
    let v2 = vertices[indices[base_idx+2]];

    let uv0 = uvs[indices[base_idx]];
    let uv1 = uvs[indices[base_idx+1]];
    let uv2 = uvs[indices[base_idx+2]];

    let dp1 = v1 - v0;
    let dp2 = v2 - v0;

    // UV deltas
    let duv1 = uv1 - uv0;
    let duv2 = uv2 - uv0;

    let r = 1.0 / (duv1.x * duv2.y - duv1.y * duv2.x);
    let T = normalize((dp1 * duv2.y - dp2 * duv1.y) * r);
    let B = normalize((dp2 * duv1.x - dp1 * duv2.x) * r);
    let N = hit.normal;

    let TBN = mat3x3<f32>(T, B, N);

    let raw = textureSampleLevel(
        textures[materials[hit.material].normal_texture],
        textures_sampler,
        hit.uv, 0.0).xyz;
    let tangent_normal = normalize(raw * 2.0 - 1.0);
    return normalize(TBN * tangent_normal);
}