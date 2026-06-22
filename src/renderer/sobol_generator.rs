use wgpu::util::DeviceExt;

use crate::renderer::GpuContext;

struct SobolParams {
    pub s: usize,
    pub a: usize,
    pub m: &'static [u32],
}

const NUM_DIMENSIONS: usize = 32;
const NUM_DIMENSIONS2: usize = NUM_DIMENSIONS*NUM_DIMENSIONS;

const SOBOL_PARAMS: [SobolParams; NUM_DIMENSIONS] = [
    SobolParams { s: 0, a: 0,  m: &[] },                          // dim 1
    SobolParams { s: 1, a: 0,  m: &[1] },                         // dim 2
    SobolParams { s: 2, a: 1,  m: &[1, 3] },                      // dim 3
    SobolParams { s: 3, a: 1,  m: &[1, 3, 1] },                   // dim 4
    SobolParams { s: 3, a: 2,  m: &[1, 1, 1] },                   // dim 5
    SobolParams { s: 4, a: 1,  m: &[1, 1, 3, 3] },                // dim 6
    SobolParams { s: 4, a: 4,  m: &[1, 3, 5, 13] },               // dim 7
    SobolParams { s: 5, a: 2,  m: &[1, 1, 5, 5, 17] },            // dim 8
    SobolParams { s: 5, a: 4,  m: &[1, 1, 5, 5, 5] },             // dim 9
    SobolParams { s: 5, a: 7,  m: &[1, 1, 7, 11, 19] },           // dim 10
    SobolParams { s: 5, a: 11, m: &[1, 1, 5, 1, 1] },             // dim 11
    SobolParams { s: 5, a: 13, m: &[1, 1, 1, 3, 11] },            // dim 12
    SobolParams { s: 5, a: 14, m: &[1, 3, 5, 5, 31] },            // dim 13
    SobolParams { s: 6, a: 1,  m: &[1, 3, 3, 9, 7, 49] },         // dim 14
    SobolParams { s: 6, a: 13, m: &[1, 1, 1, 15, 21, 21] },       // dim 15
    SobolParams { s: 6, a: 16, m: &[1, 3, 1, 13, 27, 49] },       // dim 16
    SobolParams { s: 6, a: 19, m: &[1, 1, 1, 15, 7, 5] },         // dim 17
    SobolParams { s: 6, a: 22, m: &[1, 3, 1, 15, 13, 25] },       // dim 18
    SobolParams { s: 6, a: 25, m: &[1, 1, 5, 5, 19, 61] },        // dim 19
    SobolParams { s: 7, a: 1,  m: &[1, 3, 7, 11, 23, 15, 103] },  // dim 20
    SobolParams { s: 7, a: 4,  m: &[1, 3, 7, 13, 13, 15, 69] },   // dim 21
    SobolParams { s: 7, a: 7,  m: &[1, 1, 3, 13, 7, 35, 63] },    // dim 22
    SobolParams { s: 7, a: 8,  m: &[1, 3, 5, 9, 1, 25, 53] },     // dim 23
    SobolParams { s: 7, a: 14, m: &[1, 3, 1, 13, 9, 35, 107] },   // dim 24
    SobolParams { s: 7, a: 19, m: &[1, 3, 1, 5, 27, 61, 31] },    // dim 25
    SobolParams { s: 7, a: 21, m: &[1, 1, 5, 11, 19, 41, 61] },   // dim 26
    SobolParams { s: 7, a: 28, m: &[1, 3, 5, 3, 3, 13, 69] },     // dim 27
    SobolParams { s: 7, a: 31, m: &[1, 1, 7, 13, 1, 19, 1] },     // dim 28
    SobolParams { s: 7, a: 32, m: &[1, 3, 7, 5, 13, 19, 59] },    // dim 29
    SobolParams { s: 7, a: 37, m: &[1, 1, 3, 9, 25, 29, 41] },    // dim 30
    SobolParams { s: 7, a: 41, m: &[1, 3, 5, 13, 23, 1, 55] },    // dim 31
    SobolParams { s: 7, a: 42, m: &[1, 3, 7, 3, 13, 59, 17] },    // dim 32
];

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SobolDirections {
    pub dirs: [u32; NUM_DIMENSIONS2]
}

fn generate_sobol_directions() -> SobolDirections {
    let mut result = [0u32; NUM_DIMENSIONS2];
    for i in 0..SOBOL_PARAMS.len() {
        let offset = i*NUM_DIMENSIONS;
        let SobolParams { s, a, m } = SOBOL_PARAMS[i];

        if s == 0 {
            for j in 0..NUM_DIMENSIONS {
                result[offset+j] = 1u32 << (NUM_DIMENSIONS-1 - j);
            }
            continue;
        }

        for j in 0..s {
            result[offset+j] = m[j] << (NUM_DIMENSIONS-1 - j);
        }

        for j in s..NUM_DIMENSIONS {
            let mut val = result[offset+j-s] ^ (result[offset+j-s] >> s);
            for k in 1..s {
                if (a >> (s-1-k)) & 1 == 1 {
                    val ^= result[offset+j-k];
                }
            }
            result[offset+j] = val;
        }
    }

    SobolDirections { dirs: result }
}

#[derive(Debug, Clone)]
pub struct SobolDirectionsBindGroup {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup
}

impl SobolDirectionsBindGroup {
    pub fn generate_sobol_directions_buffer(ctx: &GpuContext) -> SobolDirectionsBindGroup {
        let sobol_dirs_uniform = generate_sobol_directions();
        let sobol_dirs_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sobol Directions Buffer"),
            contents: bytemuck::cast_slice(&[sobol_dirs_uniform]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let sobol_dirs_bgl = ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sobol Directions Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ]
        });

        let sobol_dirs_bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sobol Directions Bind Group"),
            layout: &sobol_dirs_bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: sobol_dirs_buffer.as_entire_binding() }],
        });

        Self { bind_group_layout: sobol_dirs_bgl, bind_group: sobol_dirs_bind_group }
    } 
}