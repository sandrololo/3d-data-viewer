use glam::{Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;

pub struct Transformation {
    current: Mat4,
    initial: Mat4,
    initial_position: Vec3,
    pub bind_group: Option<wgpu::BindGroup>,
    buffer: Option<wgpu::Buffer>,
}

impl Default for Transformation {
    fn default() -> Self {
        Self::new()
    }
}

impl Transformation {
    pub fn new() -> Self {
        let default = Mat4::IDENTITY;
        Self {
            initial: default,
            current: default,
            initial_position: Vec3::new(0.0, 0.0, 1.0),
            bind_group: None,
            buffer: None,
        }
    }

    pub fn reset(&mut self) {
        let default = Mat4::IDENTITY;
        self.initial = default;
        self.current = default;
        self.initial_position = Vec3::new(0.0, 0.0, 1.0);
    }

    pub fn update_gpu(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            self.buffer
                .as_ref()
                .expect("Transformation buffer not initialized"),
            0,
            bytemuck::cast_slice(&self.current.to_cols_array()),
        );
    }

    pub fn start_move(&mut self, position: Vec3) {
        self.initial_position = position;
        self.initial = self.current;
    }

    pub fn rotate(&mut self, new_position: Vec3) {
        let rot_axis = self.initial_position.cross(new_position);
        let axis_len = rot_axis.length();
        let rot = mat4_from_rotation_axis(rot_axis, axis_len * 100.0);
        self.current = rot * self.initial;
    }

    pub(crate) fn create_bind_group(&mut self, device: &wgpu::Device) -> wgpu::BindGroupLayout {
        let buffer = self.create_buffer_init(device);
        let layout = Self::create_bind_group_layout(device);
        self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("transformation_range_bind_group"),
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
            label: Some("transformation_buffer"),
            contents: bytemuck::cast_slice(&self.current.to_cols_array()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("transformation_bind_group_layout"),
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

fn mat4_from_rotation_axis(axs: Vec3, phi: f32) -> Mat4 {
    let a = Vec3::normalize(axs);
    let t = phi * std::f32::consts::PI / 180.0;
    let c = f32::cos(t);
    let s = f32::sin(t);
    let d = 1.0 - c;

    let d00 = d * a[0] * a[0];
    let d01 = d * a[0] * a[1];
    let d02 = d * a[0] * a[2];
    let d11 = d * a[1] * a[1];
    let d12 = d * a[1] * a[2];
    let d22 = d * a[2] * a[2];

    let s0 = s * a[0];
    let s1 = s * a[1];
    let s2 = s * a[2];

    Mat4 {
        x_axis: Vec4::new(d00 + c, d01 - s2, d02 + s1, 0.0),
        y_axis: Vec4::new(d01 + s2, d11 + c, d12 - s0, 0.0),
        z_axis: Vec4::new(d02 - s1, d12 + s0, d22 + c, 0.0),
        w_axis: Vec4::new(0.0, 0.0, 0.0, 1.0),
    }
}
