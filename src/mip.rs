use std::num::NonZeroU32;

use wgpu::util::DeviceExt;

use crate::gpu_data::DataSize;
use crate::index_buffer::{IndexBuffer, IndexBufferBuilder};
use crate::vertex_buffer::VertexBuffer;

struct MipData {
    mip_levels: Vec<u32>,
    index_buffer: IndexBuffer,
    vertex_buffer: VertexBuffer,
    image_size: DataSize,
}

pub(crate) struct Mip {
    pub(crate) mip_buffer: wgpu::Buffer,
    pub(crate) image_dims_buffer: wgpu::Buffer,
    mip_data: Option<MipData>,
    current_level: u32,
}

impl Mip {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let mip_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mip_level_buffer"),
            contents: bytemuck::cast_slice(&[2u32]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });
        let image_dims_buffer = DataSize::create_buffer(&device);
        Self {
            mip_buffer,
            image_dims_buffer,
            mip_data: None,
            current_level: 2,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.mip_data = None;
        self.current_level = 2;
    }

    pub(crate) fn set_image(&mut self, image_size: DataSize, device: &wgpu::Device) {
        let mip_levels = (0..10u32)
            .filter(|level| {
                let num_indices = IndexBufferBuilder::triangle_strip_length(image_size, *level);
                // A higher number of indices doesn't make sense to render. 2^15 is no problem, therefore no need to go lower.
                num_indices < 2u32.pow(28) as u64 && num_indices > 2u32.pow(15) as u64
            })
            .collect();
        let index_buffer =
            IndexBufferBuilder::new_triangle_strip(image_size, &mip_levels).create_buffer(&device);
        let vertex_buffer = VertexBuffer::new(image_size, &device);
        let mip_data = MipData {
            mip_levels,
            index_buffer,
            vertex_buffer,
            image_size,
        };
        self.mip_data = Some(mip_data);
    }

    pub(crate) fn set_zoom(&mut self, zoom: f32) {
        if let Some(mip_data) = &self.mip_data {
            let levels = &mip_data.mip_levels;
            let index = ((zoom * 1.2 * levels.len() as f32) as usize)
                .min(levels.len() - 1)
                .max(0);
            self.current_level = levels[index];
        } else {
            self.current_level = 2;
        }
        log::info!("Set MIP level to: {}", self.current_level);
    }

    pub(crate) fn update_gpu(&self, renderpass: &mut wgpu::RenderPass, queue: &wgpu::Queue) {
        if let Some(mip_data) = &self.mip_data {
            let (w, h) = (
                mip_data.image_size.width.get() / 2u32.pow(self.current_level),
                mip_data.image_size.height.get() / 2u32.pow(self.current_level),
            );
            renderpass.set_vertex_buffer(
                0,
                mip_data.vertex_buffer.buffer.slice(0..(w * h * 4) as u64),
            );
            mip_data
                .index_buffer
                .set_mip_level_buffer(self.current_level, renderpass);
            queue.write_buffer(
                &self.mip_buffer,
                0,
                bytemuck::cast_slice(&[self.current_level]),
            );
            DataSize {
                width: NonZeroU32::new(w).expect("Width can't be zero"),
                height: NonZeroU32::new(h).expect("Height can't be zero"),
            }
            .write_buffer(queue, &self.image_dims_buffer);
        }
    }

    pub(crate) fn get_bind_group_entry(&self) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: 3,
            resource: self.mip_buffer.as_entire_binding(),
        }
    }
}
