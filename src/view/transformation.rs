use glam::{Mat4, Vec2};
use wgpu::{BindGroupLayout, util::DeviceExt};

/// Degrees of rotation per NDC unit of drag (the viewport spans 2 NDC units).
const DEG_PER_NDC: f32 = 90.0;
const TILT_RANGE_DEG: std::ops::RangeInclusive<f32> = 0.0..=180.0;

#[derive(Clone, Copy)]
pub struct EulerRotationDeg {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}

#[allow(dead_code)]
impl EulerRotationDeg {
    pub fn new(pitch: f32, yaw: f32, roll: f32) -> Self {
        Self { pitch, yaw, roll }
    }
}

/// Turntable rotation: tilt about X composed with an azimuth spin about the
/// height axis Z. Roll is structurally impossible.
pub struct Transformation {
    tilt_deg: f32,
    azimuth_deg: f32,
    drag_start_ndc: Vec2,
    drag_start_tilt_deg: f32,
    drag_start_azimuth_deg: f32,
    current: Mat4,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: BindGroupLayout,
    buffer: wgpu::Buffer,
}

impl Transformation {
    pub fn new(device: &wgpu::Device) -> Self {
        let default = Mat4::IDENTITY;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("transformation_buffer"),
            contents: bytemuck::cast_slice(&default.to_cols_array()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let layout = Self::create_bind_group_layout(device);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("transformation_range_bind_group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        Self {
            tilt_deg: 0.0,
            azimuth_deg: 0.0,
            drag_start_ndc: Vec2::ZERO,
            drag_start_tilt_deg: 0.0,
            drag_start_azimuth_deg: 0.0,
            current: default,
            bind_group,
            bind_group_layout: layout,
            buffer,
        }
    }

    pub fn reset(&mut self) {
        self.tilt_deg = 0.0;
        self.azimuth_deg = 0.0;
        self.current = Mat4::IDENTITY;
    }

    pub fn get_current(&self) -> Mat4 {
        self.current
    }

    pub fn update_gpu(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(&self.current.to_cols_array()),
        );
    }

    pub fn start_move(&mut self, ndc: Vec2) {
        self.drag_start_ndc = ndc;
        self.drag_start_tilt_deg = self.tilt_deg;
        self.drag_start_azimuth_deg = self.azimuth_deg;
    }

    pub fn rotate(&mut self, ndc: Vec2) {
        let delta = ndc - self.drag_start_ndc;
        self.azimuth_deg = self.drag_start_azimuth_deg + delta.x * DEG_PER_NDC;
        self.tilt_deg = (self.drag_start_tilt_deg + delta.y * DEG_PER_NDC)
            .clamp(*TILT_RANGE_DEG.start(), *TILT_RANGE_DEG.end());
        self.rebuild();
    }

    /// `pitch` maps to tilt, `roll` to azimuth; `yaw` is ignored (no roll in the
    /// turntable model).
    pub fn rotate_euler(&mut self, r: EulerRotationDeg) {
        self.tilt_deg = r
            .pitch
            .clamp(*TILT_RANGE_DEG.start(), *TILT_RANGE_DEG.end());
        self.azimuth_deg = r.roll;
        self.rebuild();
    }

    fn rebuild(&mut self) {
        self.current = Mat4::from_rotation_x(self.tilt_deg.to_radians())
            * Mat4::from_rotation_z(self.azimuth_deg.to_radians());
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
