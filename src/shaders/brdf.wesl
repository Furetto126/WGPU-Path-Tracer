import package::constants::{INFINITY, PI, INV_PI, MAX_U32};
import package::path_tracer::Ray;
import package::random;
import package::scene::{
    emissive_triangles_count, emissive_triangles,
    vertices, indices, materials, Triangle};
import package::intersection::{Hit, LightVisibilityResult, closest_intersection};

// D(h):
// Normal Distribution Function (NDFs):
//   The amount of microfacets pointing toward the half-vector.
// ------------------------------------------------------------
fn ggx_ndf(n: vec3f, h: vec3f, roughness: f32) -> f32 {
    let a2 = roughness*roughness*roughness*roughness;
    let cos_theta = max(dot(n, h), 0.0001);
    return a2*INV_PI / pow(cos_theta*cos_theta * (a2 - 1.0) + 1.0, 2.0);
}

// G(wi, wo):
// Visibility / Geometry term:
//   Attenuation factor that takes into account microfacets shadowing and blocking each other.
// -------------------------------------------------------------------------------------------
fn smith_sqrt(cos_theta: f32, roughness: f32) -> f32 {
    let a2 = roughness*roughness*roughness*roughness;
    return sqrt(a2 + (1.0 - a2)*cos_theta*cos_theta); 
}

fn g1_smith(cos_theta: f32, roughness: f32) -> f32 {
    let a2 = roughness*roughness*roughness*roughness;
    return 2.0*cos_theta / (cos_theta + smith_sqrt(cos_theta, roughness));
}

fn g2_smith(w_o: vec3f, w_i: vec3f, n: vec3f, roughness: f32) -> f32 {
    let cos_theta_wi = max(dot(n, w_i), 0.001);
    let cos_theta_wo = max(dot(n, w_o), 0.001);

    let lambda_wi = (-cos_theta_wi + smith_sqrt(cos_theta_wi, roughness)) / (2.0*cos_theta_wi);
    let lambda_wo = (-cos_theta_wo + smith_sqrt(cos_theta_wo, roughness)) / (2.0*cos_theta_wo);

    return 1.0 / (1.0 + lambda_wi + lambda_wo);
}

fn v_smith(w_o: vec3f, w_i: vec3f, normal: vec3f, roughness: f32) -> f32 {
    let cos_theta_wo = max(dot(normal, w_o), 0.001);
    let cos_theta_wi = max(dot(normal, w_i), 0.001);

    let lambda_v = cos_theta_wi*smith_sqrt(cos_theta_wo, roughness);
    let lambda_l = cos_theta_wo*smith_sqrt(cos_theta_wi, roughness);

    return 0.5 / (lambda_v + lambda_l);
}

// F(wi, h):
// Fresnel term:
//   At grazing angles everything becomes specular with color F0.
// -------------------------------------------------------------- 

/// cos_theta is dot(V, H) or dot(L, H)
fn schlick_fresnel(cos_theta: f32, f0: vec3f) -> vec3f {
    return f0 + (1.0 - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

// p(wi):
// PDF:
//   Probability that the given sample would have been picked
//   with the sampling strategy.
// ----------------------------------------------------------
fn ggx_pdf(normal: vec3f, w_o: vec3f, w_i: vec3f, roughness: f32) -> f32 {
    let h = normalize(w_i + w_o);
    let ndf = ggx_ndf(normal, h, roughness);

    let cos_theta = max(dot(normal, w_o), 0.001);
    return ndf * g1_smith(cos_theta, roughness) / (4.0*cos_theta);
}

fn lambert_pdf(normal: vec3f, w_i: vec3f) -> f32 {
    return cos(dot(normal, w_i)) * INV_PI;
}

fn light_pdf(shadow_ray: Ray, hit: Hit) -> f32 {
    let light_triangle_idx = hit.triangle_index;
    let base_vertex = light_triangle_idx * 3u;

    let v0 = vertices[indices[base_vertex]];
    let v1 = vertices[indices[base_vertex+1]];  
    let v2 = vertices[indices[base_vertex+2]];

    let ab = v0 - v1;
    let ac = v0 - v2;
    let p = cross(ab, ac);
    let area = 0.5 * length(p);
    let normal = select(normalize(p), -normalize(p), dot(shadow_ray.direction, p) > 0.0);

    let dist = hit.distance;
    let cos_theta = max(dot(-shadow_ray.direction, normal), 0.001);
    return dist*dist / (f32(emissive_triangles_count) * area * cos_theta);
}

fn light_pdf_for_direction(position: vec3f, w_i: vec3f) -> f32 {
    if emissive_triangles_count == 0 { return 0.0; }

    let ray = Ray(position, w_i, 0.001, INFINITY);
    let hit = closest_intersection(ray);

    if !hit.hasHit { return 0.0; }

    let mat = materials[hit.material];
    let em = mat.emissive_factor;
    let is_emissive = em.r > 0.0 || em.g > 0.0 || em.b > 0.0 || mat.emissive_texture != MAX_U32;

    if !is_emissive { return 0.0; }

    return light_pdf(ray, hit);
}

// f(w_o, w_i)
// BRDF:
//   How much light is transmitted along the bounce w_o -> w_i
// -----------------------------------------------------------
fn ggx_brdf(w_o: vec3f, w_i: vec3f, normal: vec3f, fresnel: vec3f, roughness: f32) -> vec3f {
    let h = normalize(w_i + w_o);

    let d = ggx_ndf(normal, h, roughness);
    let v = v_smith(w_o, w_i, normal, roughness); 

    return d * v * fresnel;
}

// Diffuse albedo is albedo * (1.0 - metallic)
fn lambert_brdf(diffuse_albedo: vec3f, fresnel: vec3f) -> vec3f {
    return (1.0 - fresnel) * diffuse_albedo * INV_PI;
}

// Samplers: Find light direction for a specific ray direction w_o.
// ----------------------------------------------------------------
// Uses Heitz 2018 VNDF
fn ggx_sample(
    N: vec3f,
    w_o: vec3f,
    roughness: f32,
    pixel_seed: u32,
    sobol_index: u32,
    sobol_dimension: ptr<function, u32>
) -> vec3f {
    var up = vec3f(0.0, 0.0, 1.0);
    if abs(N.z) > 0.9 { up = vec3f(1.0, 0.0, 0.0); }

    let T = normalize(cross(up, N));
    let B = cross(N, T);

    let v_local = vec3f(
        dot(w_o, T),
        dot(w_o, B),
        dot(w_o, N)
    );

    let a = roughness*roughness;
    let v_prime = normalize(vec3f(
        a*v_local.x,
        a*v_local.y,
        v_local.z
    ));

    var T1 = vec3f(1.0, 0.0, 0.0);
    var T2 = vec3f(0.0, 1.0, 0.0);

    if v_prime.z < 0.9999 {
        T1 = 1.0 / sqrt(v_prime.x*v_prime.x + v_prime.y*v_prime.y) * vec3f(-v_prime.y, v_prime.x, 0.0);
        T2 = cross(v_prime, T1);
    }

    let u1 = random::sobol_float(sobol_index, sobol_dimension, pixel_seed);
    let u2 = random::sobol_float(sobol_index, sobol_dimension, pixel_seed);

    let r = sqrt(u1);
    let phi = 2.0 * PI * u2;

    let t1 = r*cos(phi);
    let s = 0.5 * (1.0 + v_prime.z);
    let t2 = (1.0 - s) * sqrt(1.0 - t1*t1) + s*(r*sin(phi));

    let n_prime = t1*T1 + t2*T2 + sqrt(max(1.0 - t1*t1 - t2*t2, 0.0))*v_prime;
    let h_local = normalize(vec3f(
        a*n_prime.x,
        a*n_prime.y,
        max(n_prime.z, 0.0)
    ));

    let h_world = h_local.x*T + h_local.y*B + h_local.z*N;

    return 2.0*(dot(w_o, h_world))*h_world - w_o;
}

fn diffuse_sample(
    normal: vec3f,
    pixel_seed: u32,
    sobol_index: u32,
    sobol_dimension: ptr<function, u32>
) -> vec3f {
    let r1 = random::sobol_float(sobol_index, sobol_dimension, pixel_seed);
    let r2 = random::sobol_float(sobol_index, sobol_dimension, pixel_seed);

    let z = r1*2.0 - 1.0;
    let r = sqrt(1.0 - z*z);
    let phi = r2 * 2.0 * PI;

    return normalize(normal + vec3f(r*cos(phi), r*sin(phi), z));
}

struct LightSampleInfo {
    shadow_ray: Ray,
    triangle_idx: u32
}

fn light_sample(
    position: vec3f,
    pixel_seed: u32,
    sobol_index: u32,
    sobol_dimension: ptr<function, u32>
) -> LightSampleInfo {
    // Choose a random emissive triangle
    let r = random::sobol_float(sobol_index, sobol_dimension, pixel_seed);
    let emissive_idx = min(
        u32(r * f32(package::scene::emissive_triangles_count)),
        emissive_triangles_count - 1
    );

    let triangle_idx = emissive_triangles[emissive_idx];
    let base_vertex = triangle_idx * 3;
    let v0 = vertices[indices[base_vertex]];
    let v1 = vertices[indices[base_vertex+1]];  
    let v2 = vertices[indices[base_vertex+2]];

    let r1 = sqrt(random::sobol_float(sobol_index, sobol_dimension, pixel_seed));
    let r2 = random::sobol_float(sobol_index, sobol_dimension, pixel_seed);

    let u = 1.0 - r1;
    let v = r2 * r1;

    let light_point = u*v0 + v*v1 + (1.0-u-v)*v2;
    let ab = v0 - v1;
    let ac = v0 - v2;

    let n = normalize(cross(ab, ac));
    let normal = select(-n, n, dot(n, position - light_point) > 0.0);

    let direction = normalize((light_point + normal*0.001) - position); 

    return LightSampleInfo(
        Ray(position, direction, 0.001, INFINITY),
        triangle_idx
    );
}