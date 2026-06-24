import package::path_tracer::Ray;
import package::scene::{
    PbrMaterial, Sphere, Triangle,
    vertices, uvs, indices, triangle_materials
};
import package::constants::{EPSILON, INFINITY};

@group(2) @binding(2) var<uniform> is_scene_empty: u32;

struct Hit {
    hasHit: bool,
    distance: f32,
    point: vec3f,
    normal: vec3f,
    material: u32,
    uv: vec2f,
}

fn sphere_intersect(ray: Ray, sphere: Sphere) -> Hit {
    var hit: Hit;
    hit.hasHit = false;

    let oc = ray.origin - sphere.center;
    let b = dot(oc, ray.direction);
    let qc = oc - b*ray.direction;
    var h = sphere.radius*sphere.radius - dot(qc, qc);

    if h < 0 { return hit; }

    h = sqrt(h);
    let hits = vec2f(-b-h, -b+h); // Sphere-ray entry / exit distance

    if hits.y < EPSILON { return hit; }

    hit.hasHit = true;

    // Branch vs Branchless option, 
    // probably no performance gains after compiler optimizations :P
    // var t = hits.x;
    // if hits.x < EPSILON { t = hits.y; }
    let t = hits.x + step(hits.x, EPSILON)*2.0*h;

    hit.distance = t;
    hit.point = ray.origin + ray.direction*t;
    hit.normal = normalize(hit.point - sphere.center);
    hit.material = sphere.material;

    return hit;
}

fn triangle_intersect(ray: Ray, triangle: Triangle) -> Hit {
    var hit: Hit;
    hit.hasHit = false;

    let v1v0 = triangle.v1 - triangle.v0;
    let v2v0 = triangle.v2 - triangle.v0;
    let rov0 = ray.origin - triangle.v0;
    let n = cross(v1v0, v2v0);
    let q = cross(rov0, ray.direction);
    let d = 1.0 / dot(ray.direction, n);
    let u = d * dot(-q, v2v0);
    let v = d * dot(q, v1v0);
    let t = d * dot(-n, rov0);

    if u < 0.0 || v < 0.0 || (u+v) > 1.0 || t < EPSILON { return hit; }

    hit.hasHit = true;
    hit.distance = t;
    hit.point = ray.origin + ray.direction*t;
    hit.normal = normalize(n);

    if dot(ray.direction, hit.normal) > 0.0 {
        hit.normal = -hit.normal;
    }

    hit.material = triangle.material;
    hit.uv = vec2f(u, v);
    
    return hit;
}

fn closest_intersection(ray: Ray) -> Hit {
    var closest_hit: Hit;
    closest_hit.hasHit = false;
    closest_hit.distance = INFINITY;

    if is_scene_empty == 1 { return closest_hit; }

    for (var i: u32 = 0; i < arrayLength(&indices); i+=3 ) {
        let triangle = Triangle(
            vertices[indices[i]],
            vertices[indices[i+1]],
            vertices[indices[i+2]],
            triangle_materials[i/3u]
        );

        let current_hit = triangle_intersect(ray, triangle);
        if current_hit.hasHit && current_hit.distance < closest_hit.distance {
            closest_hit = current_hit;
            closest_hit.point += closest_hit.normal * vec3f(0.01);
            
            let w = 1.0 - current_hit.uv.x - current_hit.uv.y;
            closest_hit.uv = uvs[indices[i]] * w
               + uvs[indices[i+1]] * current_hit.uv.x
               + uvs[indices[i+2]] * current_hit.uv.y;
        }
    }

    return closest_hit;
}