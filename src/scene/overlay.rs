use futures::io;
use imask::{CreateRange, ImageDimension, ImaskSet, NonZeroRange, SortedRanges};

use crate::gpu_data::DataSize;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Region {
    pub pixels: SortedRanges<u32, u32>,
    pub color: [u8; 4],
}

impl Region {
    pub fn new(
        pixels: impl IntoIterator<Item = NonZeroRange<u64>, IntoIter: ImageDimension>,
        color: [u8; 4],
    ) -> Result<Self, io::Error> {
        let iter = pixels.into_iter();
        let roi = iter.bounds();
        let pixels =
            SortedRanges::try_from_ordered_iter(iter.map(|r| r.start..r.end).with_roi(roi))?;
        Ok(Self { pixels, color })
    }
}

pub struct OverlayTexture {
    texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub overlays: Arc<Vec<Region>>,
    size: wgpu::Extent3d,
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
        }
    }

    pub fn set_overlays(&mut self, overlays: Arc<Vec<Region>>) {
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

    fn create_overlay_data(&self) -> Vec<u8> {
        let total_pixels = (self.size.width * self.size.height) as usize;
        let mut data = vec![0u8; total_pixels * 4];

        for overlay in self.overlays.iter() {
            for span in overlay.pixels.spans::<u32>() {
                for x in span.x.start()..span.x.end() {
                    let idx = (x + span.y * self.size.width) as usize * 4;
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
