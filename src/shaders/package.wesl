import package::path_tracer;
import package::random::{PCG_Hash, sobol_float};

@group(0) @binding(0) var out_tex: texture_storage_2d<rgba32float, write>;

struct CameraUniform {
    inv_view_proj: mat4x4f,
    cam_pos: vec3f
}
@group(1) @binding(0) var<uniform> camera: CameraUniform;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3u) {
    let resolution = textureDimensions(out_tex);
    let uv = vec2f(id.xy) / vec2f(resolution);
    var ndc = uv * 2.0 - 1.0;
    let pixel_size_ndc = 2.0 / vec2f(resolution); 

    if id.x >= resolution.x || id.y >= resolution.y { return; }

    let pixel_seed = id.x ^ (id.y * PCG_Hash(id.x + id.y));
    
    var sobol_index = 0u;

    var accumulate = vec3f(0.0);
    const NUM_RAYS = 100;
    for (var i = 0u; i < NUM_RAYS; i++) {
        var sobol_dimension = 1u;
        
        let jitter_x = sobol_float(sobol_index, &sobol_dimension, pixel_seed);
        let jitter_y = sobol_float(sobol_index, &sobol_dimension, pixel_seed);
        let offset = vec2f(jitter_x, jitter_y) - 0.5;
        let jittered_ndc = ndc + (offset*pixel_size_ndc);

        var rayClip = vec4f(jittered_ndc, -1.0, 1.0);

        var rayWorld = camera.inv_view_proj * rayClip;
        rayWorld /= rayWorld.w;

        var rayDir = normalize(rayWorld.xyz - camera.cam_pos);

        var eyeRay = path_tracer::Ray(camera.cam_pos, rayDir);

        accumulate += path_tracer::trace_path(eyeRay, pixel_seed, sobol_index, sobol_dimension);

        sobol_index++;
    }

    textureStore(out_tex, vec2i(id.xy), vec4f(accumulate/NUM_RAYS, 1.0));
}