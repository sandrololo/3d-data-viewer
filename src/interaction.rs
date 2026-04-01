use std::num::NonZeroU32;

use glam::{Vec2, Vec3};
use log::error;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;
use wgpu::{Device, Queue, SurfaceTexture, TextureFormat};
use winit::{dpi::PhysicalSize, event::WindowEvent};

use crate::{
    gpu_data::{
        Capture, DataSize, pixel_picker::PixelPicker, texture_image_range::TextureImageRangeBuffer,
        topology_percentile_range::TopologyPercentileRangeBuffer,
    },
    keyboard::Keyboard,
    mip::Mip,
    mouse::Mouse,
    view::{
        projection::{Projection, Translation},
        transformation::{EulerRotationDeg, Transformation},
    },
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Orientation {
    pub zoom: f32,
    pub translation: Translation,
    pub rotation: EulerRotationDeg,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[allow(dead_code)]
impl Orientation {
    pub fn new(zoom: f32, translation: Translation, rotation: EulerRotationDeg) -> Self {
        Self {
            zoom,
            translation,
            rotation,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragMode {
    None,
    Pan,
    Rotate,
}

pub(crate) struct Interaction {
    mouse: Mouse,
    keyboard: Keyboard,
    drag_mode: DragMode,
    pub mip: Mip,
    pub transformation: Transformation,
    pub projection: Projection,
    pub pixel_picker: PixelPicker,
    pub image_capture: Capture,
    use_height_shader: bool,
    pub percentile_range_buffer: TopologyPercentileRangeBuffer,
    pub texture_range_buffer: TextureImageRangeBuffer,
}

impl Interaction {
    pub(crate) fn new(
        device: &Device,
        window_size: PhysicalSize<u32>,
        surface_format: TextureFormat,
    ) -> Self {
        let image_capture = Capture::new(
            &device,
            DataSize {
                width: NonZeroU32::new(window_size.width).expect("Windows size should not be 0"),
                height: NonZeroU32::new(window_size.height).expect("Windows size should not be 0"),
            },
            surface_format,
        );
        let transformation = Transformation::new(&device);
        let mut projection = Projection::new(&device);
        projection.update_aspect_ratio(window_size.width as f32 / window_size.height as f32);

        let percentile_range_buffer = TopologyPercentileRangeBuffer::new(&device);
        let texture_range_buffer = TextureImageRangeBuffer::new(&device);
        Self {
            mouse: Mouse::default(),
            keyboard: Keyboard::default(),
            drag_mode: DragMode::None,
            mip: Mip::new(&device),
            transformation,
            projection,
            pixel_picker: PixelPicker::new(&device, window_size),
            image_capture,
            use_height_shader: true,
            percentile_range_buffer,
            texture_range_buffer,
        }
    }

    pub(crate) fn handle_event(
        &mut self,
        event: WindowEvent,
        window_size: PhysicalSize<u32>,
        device: &Device,
    ) {
        match event {
            WindowEvent::Resized(size) => {
                self.pixel_picker.resize(device, size);
                self.projection
                    .update_aspect_ratio(size.width as f32 / size.height as f32);
                self.image_capture.resize(
                    device,
                    DataSize {
                        width: NonZeroU32::new(size.width).expect("Windows size should not be 0"),
                        height: NonZeroU32::new(size.height).expect("Windows size should not be 0"),
                    },
                );
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                self.mouse.register_move_event(position);
                self.pixel_picker.update_mouse_position(position);
                if self.mouse.is_left_button_pressed() {
                    match self.mouse.get_device_coordinates(window_size) {
                        Ok(new_position) => {
                            if self.mouse.is_pointer_inside(Vec2::from(new_position)) {
                                let target_mode = if self.keyboard.is_control_pressed() {
                                    DragMode::Pan
                                } else {
                                    DragMode::Rotate
                                };

                                if self.drag_mode != target_mode {
                                    self.drag_mode = target_mode;
                                    match self.drag_mode {
                                        DragMode::Pan => self.projection.start_move(new_position),
                                        DragMode::Rotate => self
                                            .transformation
                                            .start_move(Vec3::from((new_position, 1.0))),
                                        DragMode::None => (),
                                    }
                                } else if self.drag_mode == DragMode::Pan {
                                    self.projection.change_position(
                                        new_position,
                                        window_size.width,
                                        window_size.height,
                                    );
                                } else {
                                    self.transformation.rotate(Vec3::from((new_position, 1.0)));
                                }
                            }
                        }
                        Err(e) => error!("Failed to calculate pointer position: {}", e),
                    }
                }
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                self.mouse.register_button_event(button, state);
                if self.mouse.is_left_button_pressed() {
                    match self.mouse.get_device_coordinates(window_size) {
                        Ok(pos) => {
                            self.drag_mode = if self.keyboard.is_control_pressed() {
                                self.projection.start_move(pos);
                                DragMode::Pan
                            } else {
                                self.transformation.start_move(Vec3::from((pos, 1.0)));
                                DragMode::Rotate
                            };
                        }
                        Err(e) => error!("Failed to calculate pointer position: {}", e),
                    }
                } else {
                    self.drag_mode = DragMode::None;
                }
            }
            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase: _,
            } => {
                self.mouse.register_scroll_event(delta);
                self.projection.zoom(self.mouse.get_zoom());
                self.mip.set_zoom(self.mouse.get_zoom());
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                self.keyboard.register_event(event.clone());
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.keyboard.update_modifiers(modifiers);
            }
            _ => (),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.reset_orientation();
        self.mip.reset();
    }

    pub(crate) fn set_orientation(&mut self, orientation: Orientation) {
        self.drag_mode = DragMode::None;
        self.mouse.set_zoom(orientation.zoom);
        self.projection.zoom(orientation.zoom);
        self.mip.set_zoom(orientation.zoom);
        self.projection.move_by(orientation.translation);
        self.transformation.rotate_euler(orientation.rotation);
    }

    pub(crate) fn reset_orientation(&mut self) {
        self.projection.reset();
        self.transformation.reset();
        self.drag_mode = DragMode::None;
        self.mouse.reset_zoom();
        self.mip.set_zoom(self.mouse.get_zoom());
    }

    pub(crate) fn zoom_in(&mut self) {
        self.mouse.zoom_in();
        self.projection.zoom(self.mouse.get_zoom());
        self.mip.set_zoom(self.mouse.get_zoom());
    }

    pub(crate) fn zoom_out(&mut self) {
        self.mouse.zoom_out();
        self.projection.zoom(self.mouse.get_zoom());
        self.mip.set_zoom(self.mouse.get_zoom());
    }

    pub(crate) fn update_gpu(
        &self,
        queue: &Queue,
        encoder: &mut wgpu::CommandEncoder,
        surface_texture: &SurfaceTexture,
    ) {
        self.image_capture.copy_texture(encoder, &surface_texture);
        self.transformation.update_gpu(queue);
        self.projection.update_gpu(queue);
    }

    pub(crate) fn use_height_shader(&self) -> bool {
        self.use_height_shader
    }

    pub(crate) fn set_height_shader(&mut self) {
        self.use_height_shader = true
    }

    pub(crate) fn set_texture_shader(&mut self) {
        self.use_height_shader = false
    }
}
