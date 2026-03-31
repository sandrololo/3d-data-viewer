use std::num::NonZeroU32;

use glam::{Vec2, Vec3};
use log::error;
use wgpu::{Device, Queue, SurfaceTexture, TextureFormat};
use winit::{dpi::PhysicalSize, event::WindowEvent};

use crate::{
    gpu_data::{Capture, DataSize, pixel_picker::PixelPicker},
    keyboard::Keyboard,
    mip::Mip,
    mouse::Mouse,
    view::{projection::Projection, transformation::Transformation},
};

pub(crate) struct Interaction {
    mouse: Mouse,
    keyboard: Keyboard,
    pub mip: Mip,
    pub transformation: Transformation,
    pub projection: Projection,
    pub pixel_picker: PixelPicker,
    pub image_capture: Capture,
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
        let mut projection = Projection::default();
        projection.update_aspect_ratio(window_size.width as f32 / window_size.height as f32);
        Self {
            mouse: Mouse::new(),
            keyboard: Keyboard::new(),
            mip: Mip::new(&device),
            transformation: Transformation::default(),
            projection,
            pixel_picker: PixelPicker::new(&device, window_size),
            image_capture,
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
                                if self.keyboard.is_control_pressed() {
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
                            if self.keyboard.is_control_pressed() {
                                self.projection.start_move(pos);
                            } else {
                                self.transformation.start_move(Vec3::from((pos, 1.0)))
                            };
                        }
                        Err(e) => error!("Failed to calculate pointer position: {}", e),
                    }
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
            _ => (),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.reset_orientation();
        self.mip.reset();
    }

    pub(crate) fn reset_orientation(&mut self) {
        self.projection.reset();
        self.transformation.reset();
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
}
