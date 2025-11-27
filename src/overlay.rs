use std::ops::Range;

use crate::image::ImageSize;

pub struct Overlay {
    pub pixels: Vec<Range<u32>>,
    pub color: [u8; 4],
}

pub struct OverlayTexture<'a> {
    texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    overlays: &'a [Overlay],
    image_size: &'a ImageSize,
}

impl<'a> OverlayTexture<'a> {
    pub fn new(image_size: &'a ImageSize, overlays: &'a [Overlay], device: &wgpu::Device) -> Self {
        let texture = device.create_texture(&Self::desc(&image_size));

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture,
            view,
            image_size,
            overlays: overlays,
        }
    }

    pub fn write_to_queue(&self, queue: &wgpu::Queue) {
        let overlay_data = self.create_overlay_data();
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &overlay_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.image_size.width.get() * 4),
                rows_per_image: Some(self.image_size.height.get()),
            },
            wgpu::Extent3d {
                width: self.image_size.width.get(),
                height: self.image_size.height.get(),
                depth_or_array_layers: 1,
            },
        );
    }

    /// Creates a texture data array where each pixel (u32 index) maps to an RGBA color
    /// Returns a vec where each 4 bytes represents RGBA for that pixel index
    /// If a pixel has no overlay, it's [0, 0, 0, 0]
    fn create_overlay_data(&self) -> Vec<u8> {
        let total_pixels = (self.image_size.width.get() * self.image_size.height.get()) as usize;
        let mut data = vec![0u8; total_pixels * 4];

        for overlay in self.overlays {
            for range in &overlay.pixels {
                for pixel_idx in range.start..range.end {
                    let idx = (pixel_idx as usize) * 4;
                    if idx + 3 < data.len() {
                        data[idx] = overlay.color[0];
                        data[idx + 1] = overlay.color[1];
                        data[idx + 2] = overlay.color[2];
                        data[idx + 3] = overlay.color[3];
                    }
                }
            }
        }
        data
    }

    fn desc(image_size: &ImageSize) -> wgpu::TextureDescriptor<'static> {
        wgpu::TextureDescriptor {
            label: Some("overlay_texture"),
            size: wgpu::Extent3d {
                width: image_size.width.get(),
                height: image_size.height.get(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        }
    }
}
