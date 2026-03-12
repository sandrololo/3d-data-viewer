use futures::{FutureExt, future::Shared};
use image::{ExtendedColorType, ImageEncoder, codecs::png::CompressionType};
use std::sync::{Arc, Mutex};

use crate::gpu_data::DataSize;

pub type CaptureResult = Result<Vec<u8>, Arc<anyhow::Error>>;

pub struct Capture {
    readback_buffer: Arc<wgpu::Buffer>,
    window_size: DataSize,
    surface_format: wgpu::TextureFormat,
    /// Cached shared future - if a read is in progress, subsequent calls get the same future
    pending_read: Arc<
        Mutex<Option<Shared<std::pin::Pin<Box<dyn std::future::Future<Output = CaptureResult>>>>>>,
    >,
}

impl Capture {
    pub fn new(
        device: &wgpu::Device,
        window_size: DataSize,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            readback_buffer: Arc::new(Self::create_readback_buffer(device, &window_size)),
            window_size,
            surface_format,
            pending_read: Arc::new(Mutex::new(None)),
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, window_size: DataSize) {
        if self.window_size != window_size {
            self.window_size = window_size.clone();
            self.readback_buffer = Arc::new(Self::create_readback_buffer(device, &window_size))
        }
    }

    fn create_readback_buffer(device: &wgpu::Device, window_size: &DataSize) -> wgpu::Buffer {
        let unpadded_bytes_per_row = window_size.width.get() * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u32;
        let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;
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
        sender: futures::channel::oneshot::Sender<
            Shared<std::pin::Pin<Box<dyn std::future::Future<Output = CaptureResult>>>>,
        >,
    ) {
        sender.send(self.get(device)).unwrap();
    }

    pub fn copy_texture(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        surface_texture: &wgpu::SurfaceTexture,
    ) {
        if self.pending_read.lock().unwrap().is_some() {
            return;
        }

        let unpadded_bytes_per_row = self.window_size.width.get() * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u32;
        let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &surface_texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback_buffer,
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

    pub fn get(
        &self,
        device: Arc<wgpu::Device>,
    ) -> Shared<std::pin::Pin<Box<dyn std::future::Future<Output = CaptureResult>>>> {
        let mut pending = self.pending_read.lock().unwrap();

        // If there's already a pending read, return a clone of it
        if let Some(ref shared) = *pending {
            return shared.clone();
        }

        // Create new read future
        let buffer = self.readback_buffer.clone();
        let surface_format = self.surface_format;
        let pending_read = self.pending_read.clone();
        let window_size = self.window_size.clone();
        let (tx, rx) = async_channel::bounded::<Result<(), wgpu::BufferAsyncError>>(1);

        // Map the whole buffer slice for reading.
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.try_send(result);
            });

        let future: std::pin::Pin<Box<dyn std::future::Future<Output = CaptureResult>>> =
            Box::pin(async move {
                let _ = device.poll(wgpu::PollType::Poll);

                rx.recv()
                    .await
                    .map_err(|e| Arc::new(anyhow::anyhow!("Channel error: {:?}", e)))?
                    .map_err(|e| Arc::new(anyhow::anyhow!("Buffer map error: {:?}", e)))?;

                let slice = buffer.slice(..);
                let output_data = slice.get_mapped_range();

                // Rows are padded to COPY_BYTES_PER_ROW_ALIGNMENT; copy only the
                // unpadded bytes_per_row for each row into `rgba`.
                let bytes_per_pixel = 4usize;
                let unpadded_bytes_per_row = (window_size.width.get() as usize) * bytes_per_pixel;
                let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
                let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;

                let mut rgba =
                    Vec::with_capacity(unpadded_bytes_per_row * window_size.height.get() as usize);
                for row in 0..window_size.height.get() as usize {
                    let row_start = row * padded_bytes_per_row;
                    let row_end = row_start + unpadded_bytes_per_row;
                    rgba.extend_from_slice(&output_data[row_start..row_end]);
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

                drop(output_data);
                buffer.unmap();
                // Clear the pending read so next call starts fresh
                *pending_read.lock().unwrap() = None;
                Ok(png_encoded)
            });

        let shared = future.shared();
        *pending = Some(shared.clone());
        shared
    }
}
