import package::path_tracer::Ray;
import package::scene::{
    PbrMaterial, Sphere, Triangle, BvhNode,
    is_scene_empty, vertices, uvs, indices, bvh_triangle_indices,
    emissive_triangles_count, emissive_triangles,
    triangle_materials, bvh_nodes
};
import package::constants::{EPSILON, INFINITY};
import package::brdf::LightSampleInfo;

struct Hit {
    hasHit: bool,
    distance: f32,
    point: vec3f,
    normal: vec3f,
    material: u32,
    uv: vec2f,
    triangle_index: u32
}

/*fn sphere_intersect(ray: Ray, sphere: Sphere) -> Hit {
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
}*/

fn triangle_intersect(ray: Ray, triangle: Triangle) -> Hit {
    var hit: Hit;
    hit.hasHit = false;

    let v1v0 = triangle.v1 - triangle.v0;
    let v2v0 = triangle.v2 - triangle.v0;
    let rov0 = ray.origin - triangle.v0;
    let n = cross(v1v0, v2v0);
    let q = cross(rov0, ray.direction);
    let d = 1.0 / dot(ray.direction, n);
    let t = d * dot(-n, rov0);

    if t < ray.t_min || ray.t < t  { return hit; }

    let u = d * dot(-q, v2v0);
    let v = d * dot(q, v1v0);

    if u < 0.0 || v < 0.0 || (u+v) > 1.0 { return hit; }

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

fn aabb_intersect(ray: Ray, node: BvhNode) -> bool {
    let bmin = node.aabb_min;
    let bmax = node.aabb_max;
    let tx1 = (bmin.x - ray.origin.x) / ray.direction.x;
    let tx2 = (bmax.x - ray.origin.x) / ray.direction.x;
    var tmin = min(tx1, tx2);
    var tmax = max(tx1, tx2);
    let ty1 = (bmin.y - ray.origin.y) / ray.direction.y;
    let ty2 = (bmax.y - ray.origin.y) / ray.direction.y;
    tmin = max(tmin, min(ty1, ty2));
    tmax = min(tmax, max(ty1, ty2));
    let tz1 = (bmin.z - ray.origin.z) / ray.direction.z;
    let tz2 = (bmax.z - ray.origin.z) / ray.direction.z;
    tmin = max(tmin, min(tz1, tz2));
    tmax = min(tmax, max(tz1, tz2));
    return tmax >= tmin && tmin < ray.t && tmax > 0;
}

/*fn closest_intersection(ray_in: Ray) -> Hit {
    var closest_hit: Hit;
    closest_hit.hasHit = false;
    closest_hit.distance = INFINITY;

    if is_scene_empty == 1 { return closest_hit; }

    var ray = ray_in;
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
            closest_hit.triangle_index = i/3u;
            ray.t = closest_hit.distance;
            
            let w = 1.0 - current_hit.uv.x - current_hit.uv.y;
            closest_hit.uv = uvs[indices[i]] * w
               + uvs[indices[i+1]] * current_hit.uv.x
               + uvs[indices[i+2]] * current_hit.uv.y;
        }
    }

    return closest_hit;
}*/

fn closest_intersection(ray_in: Ray) -> Hit {
    var closest_hit: Hit;
    closest_hit.hasHit = false;
    closest_hit.distance = INFINITY;

    if is_scene_empty == 1 { return closest_hit; }

    var ray = ray_in;
    var stack: array<u32, 32>;
    var stack_ptr = 0;

    // Push root
    stack[stack_ptr] = 0u;
    stack_ptr++;

    while stack_ptr > 0 {
        stack_ptr--;
        let node_idx = stack[stack_ptr];
        let node     = bvh_nodes[node_idx];

        if !aabb_intersect(ray, node) {
            continue;
        }

        // Is not a leaf
        if node.num_triangles == 0 {
            stack[stack_ptr] = node.left_child_or_first_triangle;
            stack_ptr++;
            stack[stack_ptr] = node.left_child_or_first_triangle + 1;
            stack_ptr++;
            continue;
        }

        // Is a leaf
        for (var i = 0u; i < node.num_triangles; i++) {
            let triangle_idx = bvh_triangle_indices[node.left_child_or_first_triangle + i]; 
            let base_idx = triangle_idx * 3;

            let triangle = Triangle(
                vertices[indices[base_idx]],
                vertices[indices[base_idx+1]],
                vertices[indices[base_idx+2]],
                triangle_materials[triangle_idx]
            );

            let current_hit = triangle_intersect(ray, triangle);
            if current_hit.hasHit && current_hit.distance < closest_hit.distance {
                closest_hit = current_hit;
                closest_hit.triangle_index = triangle_idx;
                ray.t = closest_hit.distance;
                
                let w = 1.0 - current_hit.uv.x - current_hit.uv.y;
                closest_hit.uv =
                  uvs[indices[base_idx]]   * w
                + uvs[indices[base_idx+1]] * current_hit.uv.x
                + uvs[indices[base_idx+2]] * current_hit.uv.y;
            }
        }
    }

    return closest_hit;
}

struct LightVisibilityResult {
    is_visible: bool,
    hit: Hit,
    triangle: Triangle
}

fn is_light_visible(light_sample: LightSampleInfo) -> LightVisibilityResult {
    var result: LightVisibilityResult;
    result.is_visible = false;

    let triangle_idx = light_sample.triangle_idx;
    let base_vertex = triangle_idx * 3;
    let triangle = Triangle(
        vertices[indices[base_vertex]],
        vertices[indices[base_vertex+1]],
        vertices[indices[base_vertex+2]],
        triangle_materials[triangle_idx]
    );

    let direct_light_hit = triangle_intersect(light_sample.shadow_ray, triangle);
    if !direct_light_hit.hasHit { return result; }

    let closest = closest_intersection(light_sample.shadow_ray);

    if abs(direct_light_hit.distance - closest.distance) < 0.001 * closest.distance {
        result.is_visible = true;
        result.hit = closest;
        result.triangle = triangle;
    }

    return result;
}