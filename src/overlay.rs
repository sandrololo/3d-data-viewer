use std::ops::Range;

use crate::image::ImageSize;

pub struct Overlay {
    pub pixels: Vec<Range<u32>>,
    pub color: [u8; 4],
}

pub struct OverlayTexture {
    texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub overlays: Vec<Overlay>,
    size: wgpu::Extent3d,
}

impl OverlayTexture {
    pub fn new(image_size: &ImageSize, device: &wgpu::Device) -> Self {
        let size = wgpu::Extent3d {
            width: image_size.width.get(),
            height: image_size.height.get(),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&Self::desc(&size));
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture,
            view,
            overlays: Vec::new(),
            size,
        }
    }

    pub fn set_overlays(&mut self, overlays: Vec<Overlay>) {
        self.overlays = overlays;
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
                bytes_per_row: Some(self.size.width * 4),
                rows_per_image: Some(self.size.height),
            },
            self.size,
        );
    }

    /// Creates a texture data array where each pixel (u32 index) maps to an RGBA color
    /// Returns a vec where each 4 bytes represents RGBA for that pixel index
    /// If a pixel has no overlay, it's [0, 0, 0, 0]
    fn create_overlay_data(&self) -> Vec<u8> {
        let total_pixels = (self.size.width * self.size.height) as usize;
        let mut data = vec![0u8; total_pixels * 4];

        for overlay in &self.overlays {
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

    fn desc(size: &wgpu::Extent3d) -> wgpu::TextureDescriptor<'static> {
        wgpu::TextureDescriptor {
            label: Some("overlay_texture"),
            size: *size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        }
    }
}

pub fn example_overlays() -> Vec<Overlay> {
    vec![
        Overlay {
            pixels: vec![0..100, 1024..1124, 2048..2148, 3072..3172, 4096..4196],
            color: [255, 0, 0, 100],
        },
        Overlay {
            pixels: vec![5000..50000],
            color: [255, 255, 0, 100],
        },
    ]
}
