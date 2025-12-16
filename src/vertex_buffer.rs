use wgpu::util::DeviceExt;

use crate::image::ImageSize;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    vertex_id: [u32; 1],
}

pub(crate) struct VertexBuffer {
    pub buffer: wgpu::Buffer,
}

impl VertexBuffer {
    pub(crate) fn new(image_size: &ImageSize, data: &Vec<f32>, device: &wgpu::Device) -> Self {
        // Interleave z values and vertex indices into a single vertex buffer
        let mut vertices: Vec<Vertex> =
            Vec::with_capacity((image_size.width.get() * image_size.height.get()) as usize);
        for i in 0..data.len() {
            vertices.push(Vertex {
                vertex_id: [i as u32],
            });
        }
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self { buffer }
    }

    pub(crate) fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Uint32,
            }],
        }
    }
}
