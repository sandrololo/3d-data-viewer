use anyhow::anyhow;
use imbuf::Image;
use std::sync::Arc;

use crate::{SharedFuture, gpu_data::readback::GPUDataReadback};

#[derive(Clone)]
pub struct PixelValue {
    pub x: u32,
    pub y: u32,
    pub z: f32,
    pub texture: u16,
}

/// Result type for pixel reads - must be Clone for Shared futures
pub type PixelResult = Result<PixelValue, Arc<anyhow::Error>>;

pub struct PixelPicker {
    /// Texture that stores picking data (pixel_x, pixel_y) for each fragment
    picking_texture: wgpu::Texture,
    pub picking_texture_view: wgpu::TextureView,
    gpu_readback: GPUDataReadback<PixelValue>,
    /// Mouse position in physical pixels relative to the render target.
    mouse_position: (f32, f32),
    /// Render-target size in physical pixels.
    target_size: (u32, u32),
}

impl PixelPicker {
    pub const PICKING_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg32Uint;

    pub fn new(device: &wgpu::Device, target_size: (u32, u32)) -> Self {
        let (picking_texture, picking_texture_view) =
            Self::create_picking_texture(device, target_size);
        let readback_buffer = Self::create_readback_buffer(device);
        Self {
            picking_texture,
            picking_texture_view,
            gpu_readback: GPUDataReadback::new(readback_buffer),
            mouse_position: (0.0, 0.0),
            target_size,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, target_size: (u32, u32)) {
        if self.target_size != target_size {
            let (picking_texture, picking_texture_view) =
                Self::create_picking_texture(device, target_size);
            self.picking_texture = picking_texture;
            self.picking_texture_view = picking_texture_view;
            self.target_size = target_size;
        }
    }

    pub fn update_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_position = (x, y);
    }

    /// Copy the pixel at the current mouse position from the picking texture to the readback buffer.
    pub fn copy_pixel_at_mouse(&self, encoder: &mut wgpu::CommandEncoder) {
        if self.gpu_readback.has_pending_read() {
            return;
        }
        let x = (self.mouse_position.0 as u32).min(self.target_size.0.saturating_sub(1));
        let y = (self.mouse_position.1 as u32).min(self.target_size.1.saturating_sub(1));

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.picking_texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: self.gpu_readback.get_buffer(),
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn get(
        &self,
        device: Arc<wgpu::Device>,
        topology: Arc<Image<f32, 1>>,
        texture: Arc<Image<u16, 1>>,
    ) -> SharedFuture<PixelResult> {
        self.gpu_readback.get(device, move |buffer| {
            let pixel = (
                bytemuck::cast_slice::<u8, u32>(&buffer)[0],
                bytemuck::cast_slice::<u8, u32>(&buffer)[1],
            );
            let (w, h) = (topology.width().get(), topology.height().get());
            if pixel.0 >= w || pixel.1 >= h {
                return Err(Arc::new(anyhow!("Pixel out of bounds: {:?}", pixel)));
            }
            let buffer_index = pixel.1 as usize * w as usize + pixel.0 as usize;
            if buffer_index >= topology.buffer().len() || buffer_index >= texture.buffer().len() {
                return Err(Arc::new(anyhow!("Pixel out of bounds: {:?}", pixel)));
            }
            let z = topology.buffer()[buffer_index];
            Ok(PixelValue {
                x: pixel.0,
                y: pixel.1,
                z,
                texture: texture.buffer()[buffer_index],
            })
        })
    }

    fn create_picking_texture(
        device: &wgpu::Device,
        target_size: (u32, u32),
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("picking_texture"),
            size: wgpu::Extent3d {
                width: target_size.0.max(1),
                height: target_size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::PICKING_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_readback_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("picking_readback_buffer"),
            size: std::mem::size_of::<[u32; 2]>() as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        })
    }
}
