use glam::{Mat4, Vec2, Vec4};

pub struct Projection {
    initial_position: Vec2,
    initial_delta: Vec2,
    current_delta: Vec2,
    zoom: f32,
    aspect_ratio: f32,
}

impl Default for Projection {
    fn default() -> Self {
        Self::new()
    }
}

impl Projection {
    pub fn new() -> Self {
        Self {
            initial_position: Vec2::ZERO,
            initial_delta: Vec2::ZERO,
            current_delta: Vec2::ZERO,
            zoom: 2.0,
            aspect_ratio: 1.0,
        }
    }

    pub fn start_move(&mut self, position: Vec2) {
        self.initial_position = position;
        self.initial_delta = self.current_delta;
    }

    pub fn change_position(&mut self, position: Vec2) {
        self.current_delta = position - self.initial_position + self.initial_delta;
    }

    pub fn zoom(&mut self, zoom_factor: f32) {
        self.zoom = zoom_factor;
    }

    pub fn update_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
    }

    pub fn get_current(&self) -> Mat4 {
        let x_min = -self.zoom - self.current_delta.x;
        let x_max = self.zoom - self.current_delta.x;
        let y_min = -self.zoom - self.current_delta.y;
        let y_max = self.zoom - self.current_delta.y;
        let z_min = -1.0;
        let z_max = 1.0;

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
    pub(crate) fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ProjectionBuffer>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: (2 * std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 12,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: (3 * std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 13,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
