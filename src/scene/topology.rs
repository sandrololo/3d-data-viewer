use imbuf::Image;
use std::sync::Arc;

pub struct TopologyTexture {
    pub data: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub image: Arc<Image<f32, 1>>,
    size: wgpu::Extent3d,
}

impl TopologyTexture {
    pub fn new(image: Arc<Image<f32, 1>>, device: &wgpu::Device) -> Self {
        let size = wgpu::Extent3d {
            width: image.width().get(),
            height: image.height().get(),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("topology_texture"),
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
            bytemuck::cast_slice(&self.image.buffer()),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.image.width().get()),
                rows_per_image: Some(self.image.height().get()),
            },
            self.size,
        );
    }
}
