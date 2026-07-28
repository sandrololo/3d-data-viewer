use glam::Vec2;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::MouseScrollDelta,
};

/// Divisor to convert pixel-based scroll deltas to the same scale as line deltas.
const PIXEL_SCROLL_DIVISOR: f32 = 100.0;

#[derive(Default)]
pub(crate) struct Mouse {
    pub current_position: PhysicalPosition<f64>,
}

impl Mouse {
    pub(crate) fn register_move_event(&mut self, new_position: PhysicalPosition<f64>) {
        self.current_position = new_position;
    }

    pub(crate) fn scroll_delta(delta: &MouseScrollDelta) -> f32 {
        match delta {
            MouseScrollDelta::LineDelta(_delta_x, delta_y) => *delta_y,
            MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / PIXEL_SCROLL_DIVISOR,
        }
    }

    pub(crate) fn get_device_coordinates(
        &self,
        window_size: PhysicalSize<u32>,
    ) -> anyhow::Result<Vec2> {
        let w = f64::from(window_size.width - 1);
        let h = f64::from(window_size.height - 1);
        let x = (2.0 * self.current_position.x / w - 1.0) as f32;
        let y = (1.0 - 2.0 * self.current_position.y / h) as f32;
        Ok(Vec2::new(x, y))
    }

    pub(crate) fn is_pointer_inside(&self, pos: Vec2) -> bool {
        pos.x >= -1.0 && pos.x <= 1.0 && pos.y >= -1.0 && pos.y <= 1.0
    }
}
