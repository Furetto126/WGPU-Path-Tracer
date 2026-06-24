import package::constants::MAX_U32;

@group(3) @binding(0) var<storage, read> vertices: array<vec3<f32>>;
@group(3) @binding(1) var<storage, read> uvs: array<vec2<f32>>;
@group(3) @binding(2) var<storage, read> indices: array<u32>;
@group(3) @binding(3) var<storage, read> triangle_materials: array<u32>;
@group(3) @binding(4) var<storage, read> materials: array<PbrMaterial>;

// Primitives should only hold to material index!
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
    material: u32
}

struct Triangle {
    v0: vec3f,
    v1: vec3f,
    v2: vec3f,
    
    material: u32
}