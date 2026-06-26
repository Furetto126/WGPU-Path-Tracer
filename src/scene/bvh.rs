use std::ops::Div;

use cgmath::{Vector3};

use crate::scene::Model;
use crate::utils::MinMax;

#[derive(Debug, Clone, Copy)]
struct BvhTriangle {
    v0: Vector3<f32>,
    v1: Vector3<f32>,
    v2: Vector3<f32>
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct BvhNode {
    pub aabb_min: [f32; 3],
    pub left_child_or_first_triangle: u32,
    pub aabb_max: [f32; 3],
    pub num_triangles: u32
}

pub struct BVH {
    pub nodes: Vec<BvhNode>,
    root: usize,
    used: usize,
    
    triangles: Vec<BvhTriangle>,
    centroids: Vec<Vector3<f32>>,
    pub triangle_indices: Vec<u32>,
}

impl BVH {
    pub fn new(model: &Model) -> Self {
        let triangles_data = &model.triangles_data;

        let num_triangles = triangles_data.indices.len().div(3);
        let triangle_indices = (0..num_triangles as u32).collect();

        let mut triangles = vec![];
        let mut centroids = vec![];
        for v_idx in triangles_data.indices.chunks_exact(3) {
            let p0 = triangles_data.vertices[v_idx[0] as usize];
            let p1 = triangles_data.vertices[v_idx[1] as usize];
            let p2 = triangles_data.vertices[v_idx[2] as usize];

            let v0 = Vector3::new(p0[0], p0[1], p0[2]);
            let v1 = Vector3::new(p1[0], p1[1], p1[2]);
            let v2 = Vector3::new(p2[0], p2[1], p2[2]);

            triangles.push(BvhTriangle { v0, v1, v2 });
            centroids.push((v0 + v1 + v2) / 3.0);
        }

        let nodes = vec![BvhNode::default(); num_triangles * 2 - 1];

        Self {
            nodes,
            root: 0,
            used: 1,
            triangles,
            centroids,
            triangle_indices,
        }
    }

    pub fn build(&mut self) {
        let num_triangles = self.triangle_indices.len() as u32;

        self.nodes[self.root].num_triangles = num_triangles;
        self.update_bounds(self.root);
        self.subdivide_recurse(self.root);
    }

    pub fn update_bounds(&mut self, node_idx: usize) {
        let node = &mut self.nodes[node_idx];
        node.aabb_min = [1e30f32; 3];
        node.aabb_max = [-1e30f32; 3];

        let first = node.left_child_or_first_triangle as usize;
        for i in 0..node.num_triangles as usize {
            let leaf_triangle_idx = self.triangle_indices[first + i];
            let leaf = self.triangles[leaf_triangle_idx as usize];

            node.aabb_min.set_min(leaf.v0.into());
            node.aabb_min.set_min(leaf.v1.into());
            node.aabb_min.set_min(leaf.v2.into());

            node.aabb_max.set_max(leaf.v0.into());
            node.aabb_max.set_max(leaf.v1.into());
            node.aabb_max.set_max(leaf.v2.into());
        }
    }

    fn subdivide_recurse(&mut self, node_idx: usize) {
        let (first_triangle, num_triangles, aabb_min, aabb_max) = {
            let node = &self.nodes[node_idx];
            (node.left_child_or_first_triangle, node.num_triangles,
                node.aabb_min, node.aabb_max)
        };

        if num_triangles <= 2 {
            return;
        }
        
        let extent = Vector3::from(aabb_max) - Vector3::from(aabb_min);

        let mut axis = 0;
        if extent.y > extent.x { axis = 1; }
        if extent.z > extent[axis] { axis = 2; }

        // In-place partitioning
        let split_pos = aabb_min[axis] + extent[axis] * 0.5;
        let mut i = first_triangle as usize;
        let mut j = i + num_triangles as usize - 1;
        while i <= j {
            let idx = self.triangle_indices[i];
            if self.centroids[idx as usize][axis] < split_pos {
                i += 1;
                continue;
            }

            self.triangle_indices.swap(i, j);
            if j == 0 { break; }
            j -= 1;
        }

        let left_num = i as u32 - first_triangle;
        if left_num == 0 || left_num == num_triangles { return; }

        let left_child_idx = self.used;
        let right_child_idx = self.used+1;
        self.used += 2;

        self.nodes[left_child_idx].left_child_or_first_triangle = first_triangle;
        self.nodes[left_child_idx].num_triangles = left_num;
        self.nodes[right_child_idx].left_child_or_first_triangle = i as u32;
        self.nodes[right_child_idx].num_triangles = num_triangles - left_num;

        self.nodes[node_idx].left_child_or_first_triangle = left_child_idx as u32;
        self.nodes[node_idx].num_triangles = 0;

        self.update_bounds(left_child_idx);
        self.update_bounds(right_child_idx);

        self.subdivide_recurse(left_child_idx);
        self.subdivide_recurse(right_child_idx);
    }
}