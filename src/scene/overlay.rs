use crate::gpu_data::DataSize;
use std::{
    ops::{Deref, Range},
    sync::Arc,
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Copy, Clone, Debug)]
pub struct WasmBindgenPixelRange {
    start: u32,
    end: u32,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmBindgenPixelRange {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }
}

impl From<WasmBindgenPixelRange> for Range<u32> {
    fn from(range: WasmBindgenPixelRange) -> Self {
        range.start..range.end
    }
}

#[derive(Copy, Clone, Debug)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct OverlayColor([u8; 4]);

impl Deref for OverlayColor {
    type Target = [u8; 4];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl OverlayColor {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self([r, g, b, a])
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Overlay {
    pixelrange: Vec<WasmBindgenPixelRange>,
    color: OverlayColor,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl Overlay {
    pub fn new(pixelrange: Vec<WasmBindgenPixelRange>, color: OverlayColor) -> Self {
        Self { pixelrange, color }
    }
}

pub(crate) struct OverlayTexture {
    texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub overlays: Arc<Vec<Overlay>>,
    size: wgpu::Extent3d,
}

impl OverlayTexture {
    pub(crate) fn new(image_size: &DataSize, device: &wgpu::Device) -> Self {
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

    pub(crate) fn set_overlays(&mut self, overlays: Arc<Vec<Overlay>>) {
        self.overlays = overlays;
    }

    pub(crate) fn write_to_queue(&self, queue: &wgpu::Queue) {
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

        for overlay in self.overlays.iter() {
            for range in &overlay.pixelrange {
                for pixel_idx in Range::<u32>::from(*range) {
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
