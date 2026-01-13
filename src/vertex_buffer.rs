use wgpu::util::DeviceExt;

use crate::image::ImageSize;

pub(crate) struct VertexBuffer {
    pub buffer: wgpu::Buffer,
}

impl VertexBuffer {
    pub(crate) fn new(image_size: &ImageSize, device: &wgpu::Device) -> Self {
        let vertices: Vec<u32> = (0..image_size.width.get() * image_size.height.get()).collect();
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self { buffer }
    }

    pub(crate) fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<u32>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Uint32,
            }],
        }
    }
}
