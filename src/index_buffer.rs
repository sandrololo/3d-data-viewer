use wgpu::util::DeviceExt;

use crate::image::ImageSize;

pub(crate) struct IndexBuffer {
    pub buffer: wgpu::Buffer,
}

impl IndexBuffer {
    pub(crate) fn new(image_size: &ImageSize, device: &wgpu::Device) -> Self {
        let mut indices: Vec<u32> = Vec::new();
        for i in 0..image_size.height.get() - 1 {
            for j in 0..((image_size.width.get() - 1) / 2) {
                let j = j * 2;
                indices.push((i * image_size.width.get() + j) as u32);
                indices.push(((i + 1) * image_size.width.get() + j) as u32);
                indices.push((i * image_size.width.get() + j + 1) as u32);
                indices.push(((i + 1) * image_size.width.get() + j + 1) as u32);
                indices.push((i * image_size.width.get() + j + 2) as u32);
                indices.push(((i + 1) * image_size.width.get() + j + 2) as u32);
            }
        }
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        Self { buffer }
    }
}
