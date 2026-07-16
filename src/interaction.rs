use glam::{Vec2, Vec3};
use wgpu::{Device, Queue};

use crate::{
    mip::Mip,
    render::pipeline::FragmentShaderVariant,
    view::{
        projection::{Projection, Translation},
        transformation::{EulerRotationDeg, Transformation},
    },
};

const SCROLL_ZOOM_SENSITIVITY: f32 = 0.1;

#[derive(Clone, Copy)]
pub struct Orientation {
    pub zoom: f32,
    pub translation: Translation,
    pub rotation: EulerRotationDeg,
}

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

pub struct Interaction {
    drag_mode: DragMode,
    zoom_level: f32,
    pub mip: Mip,
    pub transformation: Transformation,
    pub projection: Projection,
    fragment_shader_variant: FragmentShaderVariant,
}

impl Interaction {
    pub fn new(device: &Device, aspect_ratio: f32) -> Self {
        let transformation = Transformation::new(device);
        let mut projection = Projection::new(device);
        projection.update_aspect_ratio(aspect_ratio);

        Self {
            drag_mode: DragMode::None,
            zoom_level: 1.0,
            mip: Mip::new(device),
            transformation,
            projection,
            fragment_shader_variant: FragmentShaderVariant::Height,
        }
    }

    /// Begin a drag at the given normalized device coordinate. `pan` selects panning
    /// (e.g. Ctrl/Cmd held) versus arc-ball rotation.
    pub fn begin_drag(&mut self, ndc: Vec2, pan: bool) {
        self.drag_mode = if pan {
            self.projection.start_move(ndc);
            DragMode::Pan
        } else {
            self.transformation.start_move(Vec3::from((ndc, 1.0)));
            DragMode::Rotate
        };
    }

    /// `size` is the render-target size in physical pixels.
    pub fn drag(&mut self, ndc: Vec2, size: (u32, u32)) {
        match self.drag_mode {
            DragMode::Pan => self.projection.change_position(ndc, size.0, size.1),
            DragMode::Rotate => self.transformation.rotate(Vec3::from((ndc, 1.0))),
            DragMode::None => {}
        }
    }

    pub fn end_drag(&mut self) {
        self.drag_mode = DragMode::None;
    }

    /// `delta_y` is positive when scrolling up (zoom in).
    pub fn scroll(&mut self, delta_y: f32) {
        self.zoom_level *= -SCROLL_ZOOM_SENSITIVITY * delta_y + 1.0;
        self.apply_zoom();
    }

    pub fn zoom_in(&mut self) {
        self.zoom_level *= 0.9;
        self.apply_zoom();
    }

    pub fn zoom_out(&mut self) {
        self.zoom_level *= 1.1;
        self.apply_zoom();
    }

    fn apply_zoom(&mut self) {
        self.zoom_level = self.zoom_level.clamp(0.05, 20.0);
        self.projection.zoom(self.zoom_level);
        self.mip.set_zoom(self.zoom_level);
    }

    pub fn reset(&mut self) {
        self.reset_orientation();
        self.mip.reset();
    }

    pub fn set_orientation(&mut self, orientation: Orientation) {
        self.drag_mode = DragMode::None;
        self.zoom_level = orientation.zoom;
        self.projection.zoom(orientation.zoom);
        self.mip.set_zoom(orientation.zoom);
        self.projection.move_by(orientation.translation);
        self.transformation.rotate_euler(orientation.rotation);
    }

    pub fn reset_orientation(&mut self) {
        self.projection.reset();
        self.transformation.reset();
        self.drag_mode = DragMode::None;
        self.zoom_level = 1.0;
        self.mip.set_zoom(self.zoom_level);
    }

    pub fn update_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.projection.update_aspect_ratio(aspect_ratio);
    }

    pub fn update_gpu(&self, queue: &Queue) {
        self.transformation.update_gpu(queue);
        self.projection.update_gpu(queue);
    }

    pub fn get_fragment_shader_variant(&self) -> &FragmentShaderVariant {
        &self.fragment_shader_variant
    }

    pub fn set_fragment_shader_variant(&mut self, variant: FragmentShaderVariant) {
        self.fragment_shader_variant = variant;
    }
}
