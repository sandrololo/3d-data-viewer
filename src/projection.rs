use glam::{Mat4, Vec2, Vec4};
use wgpu::util::DeviceExt;

pub struct Projection {
    initial_position: Vec2,
    initial_delta: Vec2,
    current_delta: Vec2,
    zoom: f32,
    aspect_ratio: f32,
    pub bind_group: Option<wgpu::BindGroup>,
    buffer: Option<wgpu::Buffer>,
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
            zoom: 1.0,
            aspect_ratio: 1.0,
            bind_group: None,
            buffer: None,
        }
    }

    pub fn update_gpu(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            self.buffer
                .as_ref()
                .expect("Projection buffer not initialized"),
            0,
            bytemuck::cast_slice(&self.get_current().to_cols_array()),
        );
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
        let pad3d = 3.0_f32.sqrt();
        let z_min = -pad3d;
        let z_max = pad3d;

        let mut dx = x_max - x_min;
        let mut dy = y_max - y_min;
        let dz = z_max - z_min;
        if dx <= self.aspect_ratio * dy {
            dx = dy * self.aspect_ratio;
        } else {
            dy = dx / self.aspect_ratio;
        }
        let pad_xy = 3.0_f32.sqrt();
        dx *= pad_xy;
        dy *= pad_xy;
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

    pub(crate) fn create_bind_group(&mut self, device: &wgpu::Device) -> wgpu::BindGroupLayout {
        let buffer = self.create_buffer_init(device);
        let layout = Self::create_bind_group_layout(device);
        self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("projection_range_bind_group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        }));
        self.buffer = Some(buffer);
        layout
    }

    fn create_buffer_init(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("projection_buffer"),
            contents: bytemuck::cast_slice(&self.get_current().to_cols_array()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("projection_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }
}
