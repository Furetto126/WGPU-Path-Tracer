import package::constants::MAX_U32;

@group(3) @binding(0) var<uniform> is_scene_empty: u32;
@group(3) @binding(1) var<storage, read> vertices: array<vec3<f32>>;
@group(3) @binding(2) var<storage, read> uvs: array<vec2<f32>>;
@group(3) @binding(3) var<storage, read> indices: array<u32>;
@group(3) @binding(4) var<uniform> emissive_triangles_count: u32;
@group(3) @binding(5) var<storage, read> emissive_triangles: array<u32>;
@group(3) @binding(6) var<storage, read> triangle_materials: array<u32>;
@group(3) @binding(7) var<storage, read> materials: array<PbrMaterial>;
@group(3) @binding(8) var<storage, read> bvh_nodes: array<BvhNode>;
@group(3) @binding(9) var<storage, read> bvh_triangle_indices: array<u32>;

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