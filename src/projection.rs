use glam::{Mat4, Vec4};
use winit::dpi::PhysicalSize;

pub struct Projection {
    aspect_ratio: f32,
}

impl Default for Projection {
    fn default() -> Self {
        Self::new()
    }
}

impl Projection {
    pub fn new() -> Self {
        Self { aspect_ratio: 1.0 }
    }

    pub fn update_window_size(&mut self, size: PhysicalSize<u32>) {
        self.aspect_ratio = size.width as f32 / size.height as f32;
    }

    pub fn mat4_orthographic(
        &self,
        x_min: f32,
        x_max: f32,
        y_min: f32,
        y_max: f32,
        z_min: f32,
        z_max: f32,
    ) -> Mat4 {
        let mut dx = x_max - x_min;
        let mut dy = y_max - y_min;
        let dz = z_max - z_min;
        if dx <= self.aspect_ratio * dy {
            dx = dy * self.aspect_ratio;
        } else {
            dy = dx / self.aspect_ratio;
        }
        Mat4 {
            x_axis: Vec4::new(2.0 / dx, 0.0, 0.0, 0.0),
            y_axis: Vec4::new(0.0, 2.0 / dy, 0.0, 0.0),
            z_axis: Vec4::new(0.0, 0.0, 1.0 / dz, 0.0),
            w_axis: Vec4::new(
                -(x_max + x_min) / dx,
                -(y_max + y_min) / dy,
                -z_min / dz,
                1.0,
            ),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ProjectionBuffer {
    projection: [[f32; 4]; 4],
}

impl ProjectionBuffer {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ProjectionBuffer>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: (2 * std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: (3 * std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
