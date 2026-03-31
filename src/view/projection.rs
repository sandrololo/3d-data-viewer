use glam::{Mat4, Vec2, Vec4};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;
use wgpu::util::DeviceExt;

#[derive(Clone, Copy)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Translation {
    pub x: f32,
    pub y: f32,
}

impl From<Translation> for Vec2 {
    fn from(value: Translation) -> Self {
        Vec2::new(value.x, value.y)
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl Translation {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

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
}

impl Projection {
    pub fn update_gpu(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            self.buffer
                .as_ref()
                .expect("Projection buffer not initialized"),
            0,
            bytemuck::cast_slice(&self.get_current().to_cols_array()),
        );
    }

    pub fn reset(&mut self) {
        self.initial_position = Vec2::ZERO;
        self.initial_delta = Vec2::ZERO;
        self.current_delta = Vec2::ZERO;
        self.zoom = 1.0;
    }

    pub fn move_by(&mut self, t: Translation) {
        self.current_delta = t.into();
    }

    pub fn start_move(&mut self, position: Vec2) {
        self.initial_position = position;
        self.initial_delta = self.current_delta;
    }

    fn visible_span(&self) -> Vec2 {
        let mut dx = 2.0 * self.zoom;
        let mut dy = 2.0 * self.zoom;
        if dx <= self.aspect_ratio * dy {
            dx = dy * self.aspect_ratio;
        } else {
            dy = dx / self.aspect_ratio;
        }
        // Pad the XY view to match the diagonal range used by the 3D scene.
        let pad_xy = 3.0_f32.sqrt();
        Vec2::new(dx * pad_xy, dy * pad_xy)
    }

    pub fn change_position(&mut self, position: Vec2, screen_width: u32, screen_height: u32) {
        let screen_w = screen_width.saturating_sub(1).max(1) as f32;
        let screen_h = screen_height.saturating_sub(1).max(1) as f32;
        let ndc_delta = position - self.initial_position;
        let screen_delta_px = Vec2::new(ndc_delta.x * 0.5 * screen_w, ndc_delta.y * 0.5 * screen_h);
        let view_span = self.visible_span();
        let world_delta = Vec2::new(
            screen_delta_px.x * view_span.x / screen_w,
            screen_delta_px.y * view_span.y / screen_h,
        );
        self.current_delta = world_delta + self.initial_delta;
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

        let view_span = self.visible_span();
        let dx = view_span.x;
        let dy = view_span.y;
        let dz = z_max - z_min;
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
