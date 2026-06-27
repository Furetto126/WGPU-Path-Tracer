import package::path_tracer;
import package::constants::INFINITY;
import package::random::{PCG_Hash, sobol_float};
import package::tonemap;

import package::scene::{materials};

@group(0) @binding(0) var out_tex: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var<storage, read_write> accumulation_buffer: array<vec4f>;

struct CameraUniform {
    inv_view_proj: mat4x4f,
    cam_pos: vec3f
}
@group(1) @binding(0) var<uniform> camera: CameraUniform;

struct RendererUniforms {
    frame: u32
}
@group(2) @binding(0) var<uniform> uniforms: RendererUniforms;

// In random.wesl
/*struct SobolDirections {
    dirs: array<u32, 1024>
}
@group(2) @binding(1) var<storage, read> sobol: SobolDirections;*/

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3u) {
    let resolution = textureDimensions(out_tex);
    let uv = vec2f(id.xy) / vec2f(resolution);
    
    var ndc = uv * 2.0 - 1.0;
    ndc.x *= f32(resolution.x) / f32(resolution.y);

    let pixel_size_ndc = 2.0 / f32(resolution.y); 

    if id.x >= resolution.x || id.y >= resolution.y { return; }

    // To preserve the Quasi Monte Carlo benefits of
    // using a Sobol Sequence, this seed MUST be static per-pixel.
    // It should not be altered by frame count, time, etc.,
    // nor be overwritten after hashing function calls.
    let pixel_seed = PCG_Hash(((id.x*id.x) ^ (id.y * id.x)) + id.y);

    const NUM_RAYS = 1;
    
    var sobol_index = uniforms.frame * NUM_RAYS;

    var sample_bucket = vec3f(0.0);
    for (var i = 0u; i < NUM_RAYS; i++) {
        var sobol_dimension = 0u;
        
        let jitter_x = sobol_float(sobol_index, &sobol_dimension, pixel_seed);
        let jitter_y = sobol_float(sobol_index, &sobol_dimension, pixel_seed);
        let offset = vec2f(jitter_x, jitter_y) - 0.5;
        let jittered_ndc = ndc + (offset*pixel_size_ndc);

        var rayClip = vec4f(jittered_ndc, -1.0, 1.0);

        var rayWorld = camera.inv_view_proj * rayClip;
        rayWorld /= rayWorld.w;

        var rayDir = normalize(rayWorld.xyz - camera.cam_pos);
        var eyeRay = path_tracer::Ray(camera.cam_pos, rayDir, 0.001, INFINITY);

        sample_bucket += path_tracer::trace_path(eyeRay, pixel_seed, sobol_index, sobol_dimension);

        sobol_index++;
    }

    var sample_average = sample_bucket/NUM_RAYS;
    sample_average = tonemap::aces_tonemap(sample_average);

    let pixel_idx = id.x + resolution.x * id.y;
    accumulation_buffer[pixel_idx] += vec4f(sample_average, 1.0);

    let accumulated_result = accumulation_buffer[pixel_idx].rgb / accumulation_buffer[pixel_idx].a;
    textureStore(out_tex, vec2i(id.xy), vec4f(accumulated_result, 1.0));

    // No accum shown
    //textureStore(out_tex, vec2i(id.xy), vec4f(sample_average, 1.0));
}