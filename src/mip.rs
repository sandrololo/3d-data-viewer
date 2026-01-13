use std::num::NonZeroU32;

use wgpu::util::DeviceExt;

use crate::image::ImageSize;
use crate::index_buffer::{IndexBuffer, IndexBufferBuilder};
use crate::vertex_buffer::VertexBuffer;

struct MipData {
    index_buffer: IndexBuffer,
    vertex_buffer: VertexBuffer,
    image_size: ImageSize,
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
        let image_dims_buffer = ImageSize::create_buffer(&device);
        Self {
            mip_buffer,
            image_dims_buffer,
            mip_data: None,
            current_level: 0,
        }
    }

    pub(crate) fn set_image(&mut self, image_size: &ImageSize, device: &wgpu::Device) {
        let index_buffer =
            IndexBufferBuilder::new_triangle_strip(&image_size, 3).create_buffer(&device);
        let vertex_buffer = VertexBuffer::new(&image_size, &device);
        let image_size = image_size.clone();
        let mip_data = MipData {
            index_buffer,
            vertex_buffer,
            image_size,
        };
        self.mip_data = Some(mip_data);
    }

    pub(crate) fn set_zoom(&mut self, zoom: f32) {
        self.current_level = if zoom > 0.8 {
            2u32
        } else if zoom > 0.2 {
            1u32
        } else {
            0u32
        };
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
            ImageSize {
                width: NonZeroU32::new(w).expect("Width can't be zero"),
                height: NonZeroU32::new(h).expect("Height can't be zero"),
            }
            .write_buffer(queue, &self.image_dims_buffer);
        }
    }
}
