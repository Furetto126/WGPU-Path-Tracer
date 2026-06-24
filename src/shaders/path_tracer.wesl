import package::constants::{MAX_U32, EPSILON, INV_PI};
import package::scene::{
    Sphere, Triangle, PbrMaterial, materials
};
import package::intersection::{closest_intersection};
import package::brdf;
import package::random::{PCG_Hash, random_float};

struct Ray {
    origin: vec3f,
    direction: vec3f
}

/// At each bounce we need:
///     - To find the light direction
///     - To find the PDF
///     - Get the BRDF of the Material
/// Then we compute the throughput:
///     (f_r(w_i, w_o) / p(w_i)) * L_i(w_i) * cos(theta)
fn trace_path(ray_in: Ray, pixel_seed: u32, sobol_index: u32, sobol_dim: u32) -> vec3f {
    const MAX_BOUNCES = 10000u;

    var throughput = vec3f(1.0);
    var color = vec3f(0.0);
    var ray = ray_in;
    
    var sobol_dimension = sobol_dim;
    var local_seed = pixel_seed ^ PCG_Hash(u32(abs(ray.direction.x+ray.direction.y+ray.direction.z)*f32(MAX_U32)));

    for (var i = 0u; i < MAX_BOUNCES; i++) {
        let hit = closest_intersection(ray);
        if (!hit.hasHit) {
            color += throughput * environmentalLight(ray);
            break;
        }

        let material = materials[hit.material];

        // Accumulate emission
        color += throughput * material.emissive_factor;

        // GGX Calculations
        // This can be simplified to
        //      Weight = F * G2 / G1
        // but right now is more clear
        // ---------------------------
        let normal = hit.normal;
        let roughness = material.roughness_factor;

        let w_o = -ray.direction;
        let w_i = brdf::ggx_sample(
            normal,
            w_o,
            material.roughness_factor,
            pixel_seed,
            sobol_index,
            &sobol_dimension
        );

        let cos_theta = dot(normal, w_i);
        if cos_theta <= 0.0 { break; }

        let h = normalize(w_i + w_o);
        let D = brdf::ggx_ndf(
            normal,
            h,
            roughness
        );

        let pdf = brdf::ggx_pdf(D, dot(normal, w_o), roughness);
        if pdf <= 0.0 { break; }

        let V = brdf::v_smith(
            normal,
            w_i,
            w_o,
            roughness
        );

        let f0 = mix(vec3f(0.04), material.base_color_factor.xyz, material.metallic_factor); 
        let F = brdf::schlick_fresnel(max(dot(w_o, h), 0.0), f0);

        let specular_brdf = D * V * F;

        let albedo = material.base_color_factor.xyz;
        let diffuse_brdf = albedo * INV_PI;

        // Energy reflected specularly and diffusely
        let kS = F;
        let kD = (vec3f(1.0) - kS) * (1.0 - material.metallic_factor);

        let brdf = (kD * diffuse_brdf) + specular_brdf;

        throughput *= brdf * cos_theta / pdf;

        if i >= 20 {
            let p = max(throughput.r, max(throughput.g, throughput.b));
        
            local_seed ^= PCG_Hash(local_seed);
            if (random_float(local_seed) > p) { break; }

            throughput /= p;
        }

        ray.origin = hit.point + hit.normal*EPSILON;
        ray.direction = w_i;
    }

    return color;
}

fn environmentalLight(ray: Ray) -> vec3f {
    let horizon = vec3f(0.9, 1.0, 1.0);
    let zenith = vec3f(0.37, 0.47, 0.61);
    let ground = vec3f(0.41, 0.39, 0.37);

    let a = pow(smoothstep(0.0, 0.7, ray.direction.y), 0.35);
    let gradient = mix(horizon, zenith, a);
    let b = smoothstep(-0.01, 0.0, ray.direction.y);
    return mix(ground, gradient, b);
}