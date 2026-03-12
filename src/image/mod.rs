pub(crate) use crate::image::capture::{Capture, CaptureResult};
pub(crate) use crate::image::{surface_percentile_range::*, texture_image_range::*};
use imbuf::Image;
use std::num::NonZeroU32;

mod capture;
mod surface_percentile_range;
mod texture_image_range;

pub struct SurfaceData(pub Image<f32, 1>);

impl SurfaceData {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        use std::fs::File;

        use anyhow::anyhow;
        use tiff::decoder::{Decoder, DecodingResult, Limits};

        let file = File::open(path)?;
        let mut decoder = Decoder::new(file)?.with_limits(Limits::unlimited());
        let dimensions = decoder.dimensions()?;
        let data = match decoder.read_image()? {
            DecodingResult::F32(data) => Ok(Image::<f32, 1>::new_vec(
                data,
                NonZeroU32::new(dimensions.0).ok_or(anyhow!("Invalid width"))?,
                NonZeroU32::new(dimensions.1).ok_or(anyhow!("Invalid height"))?,
            )),
            _ => Err(anyhow::anyhow!("Unsupported surface data format")),
        }?;
        Ok(Self(data))
    }
}

#[derive(Clone, PartialEq)]
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

    pub fn get_bind_group_entry(buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }
    }

    pub fn get_bind_group_layout_entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }
}
