use wgpu::{Extent3d, TextureFormat, TextureUsages, TextureView, util::DeviceExt};

use crate::renderer::GpuContext;

#[derive(Debug, Clone)]
pub struct Texture {
    texture: wgpu::Texture,
    view: TextureView,

    size: Extent3d,
    usage: TextureUsages
}

impl Texture {
    pub fn new(ctx: &GpuContext, size: Extent3d, usage: TextureUsages) -> Self {
        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Result Texture"),
            size: Extent3d {
                width: ctx.size.width,
                height: ctx.size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            size,
            usage
        }
    }

    pub fn new_with_data(ctx: &GpuContext, size: Extent3d, usage: TextureUsages, format: TextureFormat, data: &[u8]) -> Self {
        let texture = ctx.device.create_texture_with_data(
            &ctx.queue,
            &wgpu::TextureDescriptor {
                label: None,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: format,
                usage: usage,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            data
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            size,
            usage,
        }
    }

    pub fn new_dummmy(ctx: &GpuContext) -> Self {
        Self::new_with_data(ctx,
            Extent3d::default(),
            TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST, 
            TextureFormat::Rgba8UnormSrgb,
            &[0, 0, 0, 255]
        )
    }

    pub fn get_tex(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn get_view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn get_size(&self) -> Extent3d {
        self.size
    }

    // TODO: Breaks for "new_with_data" created Textures.
    pub fn resize(&mut self, ctx: &GpuContext, new_size: Extent3d) {
        *self = Texture::new(ctx, new_size, self.usage);
    }
}