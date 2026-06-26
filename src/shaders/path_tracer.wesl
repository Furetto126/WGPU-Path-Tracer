import package::constants::{MAX_U32, EPSILON, INV_PI};
import package::scene::{
    Sphere, Triangle, PbrMaterial, materials
};
import package::intersection::{closest_intersection, is_light_visible};
import package::brdf;
import package::random::{PCG_Hash, random_float, sobol_float};

struct Ray {
    origin: vec3f,
    direction: vec3f,
    t_min: f32,
    t: f32
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

    var prev_light_pdf_direction = 0.0;

    for (var bounce = 0u; bounce < MAX_BOUNCES; bounce++) {
        let hit = closest_intersection(ray);
        if (!hit.hasHit) {
            color += throughput * environmentalLight(ray);
            break;
        }

        let material = materials[hit.material];

        return material.base_color_factor.xyz;

        if prev_light_pdf_direction < 0.001 {
            color += throughput * material.emissive_factor;
        }

        let w_o = -ray.direction;
        let normal = hit.normal;
        let roughness = material.roughness_factor;

        let f0 = mix(vec3f(0.04), material.base_color_factor.xyz, material.metallic_factor); 
        let diffuse_albedo = material.base_color_factor.xyz * (1.0 - material.metallic_factor);

        // MIS weights should be:
        // a(0.5*p_cosine + 0.5*p_light) + (1 - a)*p_ggx
        let q_lambert = 0.5 * roughness;
        let q_light   = 0.5 * roughness;
        let q_ggx     = 1.0 - roughness;

        let light_sample = brdf::light_sample(
            hit.point + hit.normal*0.001,
            pixel_seed, sobol_index, &sobol_dimension
        );
        let light_visibility = is_light_visible(light_sample);
        if light_visibility.is_visible {
            let w_i_light = light_sample.shadow_ray.direction;
            let cos_theta_light = dot(normal, w_i_light);

            if cos_theta_light > 0.0 {
                let pdf_light = brdf::light_pdf(light_sample.shadow_ray, light_visibility.hit);
                let pdf_lambert_nee = brdf::lambert_pdf(normal, w_i_light);
                let pdf_ggx_nee = brdf::ggx_pdf(normal, w_o, w_i_light, roughness);
                let pdf_total_nee = q_lambert*pdf_lambert_nee + q_ggx*pdf_ggx_nee + q_light*pdf_light;

                let h_nee = normalize(w_i_light + w_o);
                let fresnel_nee = brdf::schlick_fresnel(max(dot(w_o, h_nee), 0.0), f0);
                let f_lambert_nee = brdf::lambert_brdf(diffuse_albedo, fresnel_nee);
                let f_ggx_nee = brdf::ggx_brdf(w_o, w_i_light, normal, fresnel_nee, roughness);
                let f_total_nee = f_lambert_nee + f_ggx_nee;

                let contribution = (f_total_nee * cos_theta_light) / pdf_total_nee;
                let emissive_factor = materials[light_visibility.hit.material].emissive_factor;
                color += throughput * emissive_factor * contribution;
            }
        }

        let r = sobol_float(sobol_index, &sobol_dimension, pixel_seed);

        var w_i = vec3f(0.0);
        if (r < q_lambert) {
            w_i = brdf::diffuse_sample(normal, pixel_seed, sobol_index, &sobol_dimension);
        } else {
            w_i = brdf::ggx_sample(
                normal,
                w_o,
                roughness,
                pixel_seed,
                sobol_index,
                &sobol_dimension
            );
        }

        let cos_theta = dot(normal, w_i);
        if cos_theta <= 0.0 { break; }

        let pdf_lambert = brdf::lambert_pdf(normal, w_i);
        prev_light_pdf_direction = brdf::light_pdf_for_direction(hit.point, w_i);
        let pdf_ggx = brdf::ggx_pdf(normal, w_o, w_i, roughness);
        let pdf_total = q_lambert*pdf_lambert + q_ggx*pdf_ggx + q_light*prev_light_pdf_direction;

        if pdf_total <= 0.0 { break; }

        let h = normalize(w_i + w_o);
        let fresnel = brdf::schlick_fresnel(max(dot(w_o, h), 0.0), f0);
        let f_lambert = brdf::lambert_brdf(diffuse_albedo, fresnel);
        let f_ggx     = brdf::ggx_brdf(w_o, w_i, normal, fresnel, roughness);

        let f_total = f_lambert + f_ggx;

        throughput *= f_total * cos_theta / pdf_total;

        if bounce >= 20 {
            let p = max(throughput.r, max(throughput.g, throughput.b));
        
            local_seed ^= PCG_Hash(local_seed);
            if (random_float(local_seed) > p) { break; }

            throughput /= p;
        }

        ray.origin = hit.point + hit.normal*0.001;
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