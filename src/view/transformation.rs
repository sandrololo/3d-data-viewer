use glam::{Mat4, Vec3};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;
use wgpu::{BindGroupLayout, util::DeviceExt};

#[derive(Clone, Copy)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct EulerRotationDeg {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[allow(dead_code)]
impl EulerRotationDeg {
    pub fn new(pitch: f32, yaw: f32, roll: f32) -> Self {
        Self { pitch, yaw, roll }
    }
}
pub(crate) struct Transformation {
    current: Mat4,
    initial: Mat4,
    initial_position: Vec3,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: BindGroupLayout,
    buffer: wgpu::Buffer,
}

impl Transformation {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
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
            initial: default,
            current: default,
            initial_position: Vec3::new(0.0, 0.0, 1.0),
            bind_group,
            bind_group_layout: layout,
            buffer,
        }
    }

    pub(crate) fn reset(&mut self) {
        let default = Mat4::IDENTITY;
        self.initial = default;
        self.current = default;
        self.initial_position = Vec3::new(0.0, 0.0, 1.0);
    }

    pub(crate) fn update_gpu(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(&self.current.to_cols_array()),
        );
    }

    pub(crate) fn start_move(&mut self, position: Vec3) {
        self.initial_position = position;
        self.initial = self.current;
    }

    pub(crate) fn rotate(&mut self, new_position: Vec3) {
        if self.initial_position != new_position {
            let rot_axis = self.initial_position.cross(new_position);
            // Axis length represents the mouse distance moved and is multiplied by constant *100
            let rot = Mat4::from_axis_angle(
                -Vec3::normalize(rot_axis),
                rot_axis.length() * 100.0 * std::f32::consts::PI / 180.0,
            );
            self.current = rot * self.initial;
        }
    }

    pub(crate) fn rotate_euler(&mut self, r: EulerRotationDeg) {
        let pitch = r.pitch * std::f32::consts::PI / 180.0;
        let yaw = r.yaw * std::f32::consts::PI / 180.0;
        let roll = r.roll * std::f32::consts::PI / 180.0;
        let rotation = Mat4::from_euler(glam::EulerRot::XYZ, pitch, yaw, roll);
        self.current = rotation;
        self.initial = rotation;
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
