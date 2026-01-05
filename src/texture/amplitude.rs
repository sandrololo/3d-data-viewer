use crate::image::{Image, ImageSize};

pub struct AmplitudeTexture {
    pub data: wgpu::Texture,
    pub view: wgpu::TextureView,
    image: Option<Image<u16>>,
    size: wgpu::Extent3d,
}

impl AmplitudeTexture {
    pub fn new(image_size: &ImageSize, device: &wgpu::Device) -> Self {
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
            label: Some("amplitude_texture"),
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

    pub fn set_image(&mut self, image: Image<u16>) {
        self.image = Some(image);
    }

    pub fn write_to_queue(&self, queue: &wgpu::Queue) {
        if let Some(image) = &self.image {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.data,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(&image.data),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(2 * image.size.width.get()),
                    rows_per_image: Some(image.size.height.get()),
                },
                self.size,
            );
        }
    }
}
