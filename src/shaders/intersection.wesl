import package::path_tracer::Ray;
import package::primitives::{Material, Sphere, Triangle, SPHERES, TRIANGLES};
import package::constants::{EPSILON, INFINITY};

struct Hit {
    hasHit: bool,
    distance: f32,
    point: vec3f,
    normal: vec3f,
    material: Material
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
    hit.material = sphere.material;

    // Branch vs Branchless option, 
    // probably no performance gains after compiler optimizations :P
    // var t = hits.x;
    // if hits.x < EPSILON { t = hits.y; }
    let t = hits.x + step(hits.x, EPSILON)*2.0*h;

    hit.distance = t;
    hit.point = ray.origin + ray.direction*t;
    hit.normal = normalize(hit.point - sphere.center);

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
    hit.material = triangle.material;
    
    return hit;
}

fn closest_intersection(ray: Ray/*, spheres: array<Sphere>, triangles: array<Triangle>*/) -> Hit {
    var closest_hit: Hit;
    closest_hit.hasHit = false;
    closest_hit.distance = INFINITY;

    for (var i: u32 = 0; i < arrayLength(SPHERES); i++ ) {
        let sphere = SPHERES[i];
        let current_hit = sphere_intersect(ray, sphere);

        if current_hit.hasHit && current_hit.distance < closest_hit.distance {
            closest_hit = current_hit;
            closest_hit.point += closest_hit.normal * vec3f(0.01);
        }
    }

    for (var i: u32 = 0; i < arrayLength(TRIANGLES); i++ ) {
        let triangle = TRIANGLES[i];
        let current_hit = triangle_intersect(ray, triangle);

        if current_hit.hasHit && current_hit.distance < closest_hit.distance {
            closest_hit = current_hit;
            closest_hit.point += closest_hit.normal * vec3f(0.01);
        }
    }

    return closest_hit;
}

fn closest_intersection_fixed(ray: Ray) -> Hit {
    var closest_hit: Hit;
    closest_hit.hasHit = false;
    closest_hit.distance = INFINITY;

    for (var i = 0u; i < 5; i++ ) {
        let sphere = SPHERES[i];
        let current_hit = sphere_intersect(ray, sphere);

        if current_hit.hasHit && current_hit.distance < closest_hit.distance {
            closest_hit = current_hit;
            closest_hit.point += closest_hit.normal * vec3f(0.01);
        }
    }

    for (var i = 0u; i < 2; i++ ) {
        let triangle = TRIANGLES[i];
        let current_hit = triangle_intersect(ray, triangle);

        if current_hit.hasHit && current_hit.distance < closest_hit.distance {
            closest_hit = current_hit;
            closest_hit.point += closest_hit.normal * vec3f(0.01);
        }
    }

    return closest_hit;
}