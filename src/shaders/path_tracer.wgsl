@group(0) @binding(0) var out_tex: texture_storage_2d<rgba32float, write>;

struct CameraUniform {
    inv_view_proj: mat4x4<f32>,
    cam_pos: vec3<f32>
};
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let resolution = textureDimensions(out_tex);
    let uv = vec2<f32>(id.xy) / vec2<f32>(resolution);

    if id.x >= resolution.x || id.y >= resolution.y { return; }

    var ndc = uv * 2.0 - 1.0;

    var rayClip = vec4<f32>(ndc, -1.0, 1.0);
    var rayWorld = camera.inv_view_proj * rayClip;
    rayWorld /= rayWorld.w;

    var rayDir = normalize(rayWorld.xyz - camera.cam_pos);
    textureStore(out_tex, vec2<i32>(id.xy), vec4<f32>(rayDir, 1.0));
}