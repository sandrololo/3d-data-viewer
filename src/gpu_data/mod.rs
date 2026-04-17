pub(crate) use crate::gpu_data::capture::{Capture, CaptureResult};
use imbuf::Image;
use std::num::NonZeroU32;

mod capture;
pub(crate) mod pixel_picker;
mod readback;
pub(crate) mod texture_image_range;
pub(crate) mod topology_percentile_range;

pub(crate) struct TopologyData(pub Image<f32, 1>);

impl TopologyData {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path)?;
        Ok(Self(crate::tiff_decode::decode_tiff(file)?))
    }
}

#[derive(Copy, Clone, PartialEq)]
pub(crate) struct DataSize {
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
    pub(crate) fn create_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image_dims_buffer"),
            size: std::mem::size_of::<[u32; 2]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub(crate) fn write_buffer(&self, queue: &wgpu::Queue, buffer: &wgpu::Buffer) {
        queue.write_buffer(
            buffer,
            0,
            bytemuck::cast_slice(&[self.width.get(), self.height.get()]),
        );
    }

    pub(crate) fn get_bind_group_entry(buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }
    }
}
