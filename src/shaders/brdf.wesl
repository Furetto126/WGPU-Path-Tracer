import package::constants::{PI, INV_PI};
import package::path_tracer::Ray;
import package::random;

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

fn g2_smith(n: vec3f, w_i: vec3f, w_o: vec3f, roughness: f32) -> f32 {
    let cos_theta_wi = max(dot(n, w_i), 0.001);
    let cos_theta_wo = max(dot(n, w_o), 0.001);

    let lambda_wi = (-cos_theta_wi + smith_sqrt(cos_theta_wi, roughness)) / (2.0*cos_theta_wi);
    let lambda_wo = (-cos_theta_wo + smith_sqrt(cos_theta_wo, roughness)) / (2.0*cos_theta_wo);

    return 1.0 / (1.0 + lambda_wi + lambda_wo);
}

fn v_smith(n: vec3f, w_i: vec3f, w_o: vec3f, roughness: f32) -> f32 {
    let cos_theta_wi = max(dot(n, w_i), 0.001);
    let cos_theta_wo = max(dot(n, w_o), 0.001);

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
//   I first have to understand it properly :P
// -------------------------------------------
fn ggx_pdf(ggx_ndf: f32, n_dot_wo: f32, roughness: f32) -> f32 {
    return ggx_ndf * g1_smith(n_dot_wo, roughness) / (4.0*n_dot_wo);
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
) -> vec3f 
{
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