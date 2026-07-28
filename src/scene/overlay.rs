use imask::{CreateRange, SortedRanges};

use crate::gpu_data::DataSize;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Region {
    pub pixels: SortedRanges<u32, u32>,
    pub color: [u8; 4],
}

impl Region {
    pub fn new(pixels: SortedRanges<u32, u32>, color: [u8; 4]) -> Self {
        Self { pixels, color }
    }
}

pub struct OverlayTexture {
    texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub overlays: Arc<Vec<Region>>,
    size: wgpu::Extent3d,
    active_region: Option<usize>,
    default_opacity: u8,
    active_opacity: u8,
}

impl OverlayTexture {
    pub fn new(image_size: &DataSize, device: &wgpu::Device) -> Self {
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
            overlays: Arc::new(Vec::new()),
            size,
            active_region: None,
            default_opacity: 128,
            active_opacity: 200,
        }
    }

    pub fn set_overlays(&mut self, overlays: Arc<Vec<Region>>) {
        self.overlays = overlays;
    }

    pub fn set_default_opacity(&mut self, opacity: u8) {
        self.default_opacity = opacity;
    }

    pub fn set_active_opacity(&mut self, opacity: u8) {
        self.active_opacity = opacity;
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

    fn create_overlay_data(&self) -> Vec<u8> {
        let total_pixels = (self.size.width * self.size.height) as usize;
        let mut data = vec![0u8; total_pixels * 4];

        for (i, overlay) in self.overlays.iter().enumerate() {
            for span in overlay.pixels.spans::<u32>() {
                for x in span.x.start()..span.x.end() {
                    let is_active = self.active_region == Some(i);
                    let opacity = self.opacity(is_active);
                    let [r, g, b, a] = overlay.color;
                    let idx = (x + span.y * self.size.width) as usize * 4;
                    let a = (a as u16 * opacity as u16 / 255) as u8;
                    if idx + 3 < data.len() {
                        data[idx] = r;
                        data[idx + 1] = g;
                        data[idx + 2] = b;
                        data[idx + 3] = a;
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

    fn opacity(&self, is_active: bool) -> u8 {
        if is_active {
            self.active_opacity
        } else {
            self.default_opacity
        }
    }
}
