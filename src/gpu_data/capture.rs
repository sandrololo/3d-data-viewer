use image::{ExtendedColorType, ImageEncoder, codecs::png::CompressionType};
use std::sync::Arc;

use crate::{
    SharedFuture,
    gpu_data::{DataSize, readback::GPUDataReadback},
};

pub type CaptureResult = Result<Vec<u8>, Arc<anyhow::Error>>;

pub struct Capture {
    gpu_readback: GPUDataReadback<Vec<u8>>,
    window_size: DataSize,
    surface_format: wgpu::TextureFormat,
}

impl Capture {
    pub fn new(
        device: &wgpu::Device,
        window_size: DataSize,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            gpu_readback: GPUDataReadback::new(Self::create_readback_buffer(device, &window_size)),
            window_size,
            surface_format,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, window_size: DataSize) {
        if self.window_size != window_size {
            self.gpu_readback
                .set_buffer(Self::create_readback_buffer(device, &window_size));
            self.window_size = window_size;
        }
    }

    fn create_readback_buffer(device: &wgpu::Device, window_size: &DataSize) -> wgpu::Buffer {
        let unpadded_bytes_per_row = window_size.width.get() * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
        let size = padded_bytes_per_row as u64 * window_size.height.get() as u64;

        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("capture_readback_buffer"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        })
    }

    #[allow(dead_code)]
    pub fn write_to_channel(
        &self,
        device: Arc<wgpu::Device>,
        sender: futures::channel::oneshot::Sender<SharedFuture<CaptureResult>>,
    ) {
        sender.send(self.get(device)).unwrap();
    }

    pub fn copy_texture(&self, encoder: &mut wgpu::CommandEncoder, color_texture: &wgpu::Texture) {
        if self.gpu_readback.has_pending_read() {
            return;
        }

        let unpadded_bytes_per_row = self.window_size.width.get() * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: self.gpu_readback.get_buffer(),
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(self.window_size.height.get()),
                },
            },
            wgpu::Extent3d {
                width: self.window_size.width.get(),
                height: self.window_size.height.get(),
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn get(&self, device: Arc<wgpu::Device>) -> SharedFuture<CaptureResult> {
        let surface_format = self.surface_format;
        let window_size = self.window_size;
        self.gpu_readback.get(device, move |buffer| {
            // Rows are padded to COPY_BYTES_PER_ROW_ALIGNMENT; copy only the
            // unpadded bytes_per_row for each row into `rgba`.
            let bytes_per_pixel = 4usize;
            let unpadded_bytes_per_row = (window_size.width.get() as usize) * bytes_per_pixel;
            let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
            let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;

            let mut rgba =
                Vec::with_capacity(unpadded_bytes_per_row * window_size.height.get() as usize);
            for row in 0..window_size.height.get() as usize {
                let row_start = row * padded_bytes_per_row;
                let row_end = row_start + unpadded_bytes_per_row;
                rgba.extend_from_slice(&buffer[row_start..row_end]);
            }

            // Convert BGRA->RGBA if needed
            match surface_format {
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
                    for i in (0..rgba.len()).step_by(4) {
                        rgba.swap(i, i + 2);
                    }
                }
                _ => {}
            }

            let mut png_encoded = Vec::new();
            let encoder = ::image::codecs::png::PngEncoder::new_with_quality(
                &mut png_encoded,
                CompressionType::Fast,
                ::image::codecs::png::FilterType::default(),
            );
            encoder
                .write_image(
                    &rgba,
                    window_size.width.get(),
                    window_size.height.get(),
                    ExtendedColorType::Rgba8,
                )
                .map_err(|e| Arc::new(anyhow::anyhow!("PNG encoding error: {:?}", e)))?;
            Ok(png_encoded)
        })
    }
}
