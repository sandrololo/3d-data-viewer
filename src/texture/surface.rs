use std::num::NonZeroU32;

use crate::image::{Image, ImageSize};

pub struct SurfaceTexture {
    pub data: wgpu::Texture,
    pub view: wgpu::TextureView,
    image: Image<f32>,
    size: wgpu::Extent3d,
}

impl SurfaceTexture {
    pub fn new(image: Image<f32>, device: &wgpu::Device) -> Self {
        let size = wgpu::Extent3d {
            width: image.size.width.get(),
            height: image.size.height.get(),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 3,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("surface_texture"),
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            data: texture,
            view,
            image,
            size,
        }
    }

    pub fn write_to_queue(&self, queue: &wgpu::Queue) {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.data,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&self.image.data),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.image.size.width.get()),
                rows_per_image: Some(self.image.size.height.get()),
            },
            self.size,
        );
        let mip_level_1_size = ImageSize {
            width: NonZeroU32::new((self.image.size.width.get() / 2).max(1)).unwrap(),
            height: NonZeroU32::new((self.image.size.height.get() / 2).max(1)).unwrap(),
        };
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.data,
                mip_level: 1,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&self.image.resize(&mip_level_1_size).data),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * mip_level_1_size.width.get()),
                rows_per_image: Some(mip_level_1_size.height.get()),
            },
            wgpu::Extent3d {
                width: mip_level_1_size.width.get(),
                height: mip_level_1_size.height.get(),
                depth_or_array_layers: 1,
            },
        );
        let mip_level_2_size = ImageSize {
            width: NonZeroU32::new((self.image.size.width.get() / 4).max(1)).unwrap(),
            height: NonZeroU32::new((self.image.size.height.get() / 4).max(1)).unwrap(),
        };
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.data,
                mip_level: 2,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&self.image.resize(&mip_level_2_size).data),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * mip_level_2_size.width.get()),
                rows_per_image: Some(mip_level_2_size.height.get()),
            },
            wgpu::Extent3d {
                width: mip_level_2_size.width.get(),
                height: mip_level_2_size.height.get(),
                depth_or_array_layers: 1,
            },
        );
    }
}
