import package::constants::MAX_U32;

// Primitives should only hold to material index!
struct Material {
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
    material: Material
}

struct Triangle {
    v0: vec3f,
    v1: vec3f,
    v2: vec3f,
    material: Material
}


// Global compile-time constant array of Spheres
const SPHERES = array(
    // 1. Top-Left: Pure Dielectric, Smooth (Deep Red)
    Sphere(
        vec3f(-1.0, 0.6, -3.0), 0.4, 
        Material(vec4f(0.7, 0.1, 0.1, 1.0), MAX_U32, 0.0, 0.05, MAX_U32, vec3f(0.0), MAX_U32, MAX_U32)
    ),
    
    // 2. Top-Right: Pure Dielectric, Medium Roughness (Emerald Green)
    Sphere(
        vec3f(1.0, 0.6, -3.0), 0.4, 
        Material(vec4f(0.1, 0.7, 0.1, 1.0), MAX_U32, 0.0, 0.35, MAX_U32, vec3f(0.0), MAX_U32, MAX_U32)
    ),
    
    // 3. Bottom-Left: Semi-Metallic, Rough (Light Blue)
    Sphere(
        vec3f(-1.2, -0.4, -3.0), 0.4, 
        Material(vec4f(0.2, 0.5, 1.0, 1.0), MAX_U32, 0.5, 0.5, MAX_U32, vec3f(0.0), MAX_U32, MAX_U32)
    ),
    
    // 4. Bottom-Center: Pure Polished Metal (Copper / Rose Gold)
    Sphere(
        vec3f(0.0, -0.4, -2.5), 0.4, 
        Material(vec4f(0.95, 0.64, 0.54, 1.0), MAX_U32, 1.0, 0.1, MAX_U32, vec3f(0.0), MAX_U32, MAX_U32)
    ),
    
    // 5. Bottom-Right: Pure Rough Metal (Silver / Aluminum)
    Sphere(
        vec3f(1.2, -0.4, -3.0), 0.4, 
        Material(vec4f(0.91, 0.92, 0.92, 1.0), MAX_U32, 1.0, 0.6, MAX_U32, vec3f(0.0), MAX_U32, MAX_U32)
    )
);

// --- CONST TRIANGLES ARRAY ---
const TRIANGLES = array(
    // Triangle 1: Massive, rough matte dark grey floor
    Triangle(
        vec3f(-10.0, -1.0, -1.0),
        vec3f( 10.0, -1.0, -1.0),
        vec3f(  0.0, -1.0, -10.0),
        Material(vec4f(0.2, 0.2, 0.2, 1.0), MAX_U32, 0.0, 0.7, MAX_U32, vec3f(0.0), MAX_U32, MAX_U32)
    ),

    // Triangle 2: Large back wall backdrop to catch bounced light/shadows
    Triangle(
        vec3f(-5.0, -1.0, -5.0),
        vec3f( 5.0, -1.0, -5.0),
        vec3f( 0.0,  4.0, -5.0),
        Material(vec4f(0.8, 0.8, 0.8, 1.0), MAX_U32, 0.1, 0.2, MAX_U32, vec3f(0.0), MAX_U32, MAX_U32)
    )
);