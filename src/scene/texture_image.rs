use crate::gpu_data::DataSize;
use imbuf::Image;
use std::sync::Arc;

pub(crate) struct TextureImage {
    pub data: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub image: Option<Arc<Image<u16, 1>>>,
    size: wgpu::Extent3d,
}

impl TextureImage {
    pub(crate) fn new(image_size: &DataSize, device: &wgpu::Device) -> Self {
        let size = wgpu::Extent3d {
            width: image_size.width.get(),
            height: image_size.height.get(),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R16Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("texture"),
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            data: texture,
            view,
            image: None,
            size,
        }
    }

    pub(crate) fn set_image(&mut self, image: Image<u16, 1>) {
        self.image = Some(Arc::new(image));
    }

    pub(crate) fn write_to_queue(&self, queue: &wgpu::Queue) {
        if let Some(image) = self.image.as_ref() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.data,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(image.buffer()),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(2 * image.width().get()),
                    rows_per_image: Some(image.height().get()),
                },
                self.size,
            );
        }
    }
}
