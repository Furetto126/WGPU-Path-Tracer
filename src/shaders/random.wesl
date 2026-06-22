import package::constants::INV_MAX_U32;

struct SobolDirections {
    dirs: array<u32, 1024>
}
@group(2) @binding(0) var<storage, read> sobol: SobolDirections;

fn PCG_Hash(seed: u32) -> u32 {
    let state = seed * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn PCG3D_Hash(vIn: vec3u) -> vec3u {
    var v = vIn * 1664525u + 1013904223u;
    v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
    v ^= v >> vec3u(16u);
    v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
    return v;
}

// Laine-Karras Owen scrambling approximation.
fn lk_hash(value: u32, dimension_seed: u32) -> u32 {
    var x = value + dimension_seed;
    x ^= x * 0x6c50b47cu;
    x ^= x * 0xb82f1e52u;
    x ^= x * 0xc7afe638u;
    x ^= x * 0x8d22f6e6u;
    return x;
}

fn random_float(seed: u32) -> f32 {
    let hash = PCG_Hash(seed);
    return f32(hash) * INV_MAX_U32;
}

fn sobol_sample(index: u32, dim: u32) -> u32 {
    var result = 0u;
    let base = dim*32;
    for (var b = 0u; b < 32; b++) {
        if ((index >> b) & 1) != 0 {
            result ^= sobol.dirs[base+b];
        }
    }

    return result;
}

fn sobol_float(index: u32, dim: ptr<function, u32>, seed: u32) -> f32 {
    let dim_seed = seed ^ (*dim * 0x9e3779b9u);
    let raw_sobol = sobol_sample(index, *dim);

    let reversed       = reverseBits(raw_sobol);
    let hashed         = lk_hash(reversed, dim_seed);
    let scrambled_bits = reverseBits(hashed);

    *dim += 1;

    return f32(scrambled_bits) * INV_MAX_U32;
}

fn sobol_vec3f(index: u32, dim: ptr<function, u32>, seed: u32) -> vec3f {
    let x = sobol_float(index, dim, seed);
    let y = sobol_float(index, dim, seed);
    let z = sobol_float(index, dim, seed);
    
    return vec3f(x, y, z);
}