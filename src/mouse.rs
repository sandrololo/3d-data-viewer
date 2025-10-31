use glam::{Vec2, Vec3};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
};

pub struct Mouse {
    current_position: PhysicalPosition<f64>,
    left_button: ElementState,
    current_zoom: f32,
}

impl Default for Mouse {
    fn default() -> Self {
        Self::new()
    }
}

impl Mouse {
    pub fn new() -> Self {
        Self {
            current_position: PhysicalPosition::new(0.0, 0.0),
            left_button: ElementState::Released,
            current_zoom: 2.0,
        }
    }

    pub fn register_button_event(&mut self, button: MouseButton, state: ElementState) {
        match button {
            MouseButton::Left => {
                if state == ElementState::Pressed {
                    self.left_button = ElementState::Pressed;
                } else {
                    self.left_button = ElementState::Released;
                }
            }
            _ => (),
        }
    }

    pub fn register_move_event(&mut self, new_position: PhysicalPosition<f64>) {
        self.current_position = new_position;
    }

    pub fn register_scroll_event(&mut self, delta: MouseScrollDelta) {
        match delta {
            MouseScrollDelta::LineDelta(_delta_x, delta_y) => {
                self.current_zoom *= -0.1 * delta_y + 1.0;
            }
            _ => (),
        }
    }

    pub fn get_device_coordinates(&self, window_size: PhysicalSize<u32>) -> anyhow::Result<Vec2> {
        let w = f64::try_from(window_size.width - 1)?;
        let h = f64::try_from(window_size.height - 1)?;
        let x = (2.0 * self.current_position.x / w - 1.0) as f32;
        let y = (1.0 - 2.0 * self.current_position.y / h) as f32;
        Ok(Vec2::new(x, y))
    }

    pub fn is_left_button_pressed(&self) -> bool {
        self.left_button == ElementState::Pressed
    }

    pub fn get_zoom(&self) -> f32 {
        self.current_zoom
    }

    pub fn is_pointer_inside(&self, pos: Vec3) -> bool {
        pos.x >= -1.0 && pos.x <= 1.0 && pos.y >= -1.0 && pos.y <= 1.0
    }
}
