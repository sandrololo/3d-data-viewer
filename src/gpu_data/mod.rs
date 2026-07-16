pub use crate::gpu_data::capture::{Capture, CaptureResult};
use std::num::NonZeroU32;

mod capture;
pub mod pixel_picker;
mod readback;
pub mod texture_image_range;
pub mod topology_percentile_range;

#[derive(Copy, Clone, PartialEq)]
pub struct DataSize {
    pub width: NonZeroU32,
    pub height: NonZeroU32,
}

impl From<(NonZeroU32, NonZeroU32)> for DataSize {
    fn from(value: (NonZeroU32, NonZeroU32)) -> Self {
        Self {
            width: value.0,
            height: value.1,
        }
    }
}

impl DataSize {
    pub fn create_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image_dims_buffer"),
            size: std::mem::size_of::<[u32; 2]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn write_buffer(&self, queue: &wgpu::Queue, buffer: &wgpu::Buffer) {
        queue.write_buffer(
            buffer,
            0,
            bytemuck::cast_slice(&[self.width.get(), self.height.get()]),
        );
    }

    pub fn get_bind_group_entry(buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }
    }
}
