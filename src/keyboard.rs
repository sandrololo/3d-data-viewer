use winit::event::ElementState;

pub struct Keyboard {
    control_button: ElementState,
}

impl Default for Keyboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Keyboard {
    pub fn new() -> Self {
        Self {
            control_button: ElementState::Released,
        }
    }

    pub fn is_control_pressed(&self) -> bool {
        self.control_button == ElementState::Pressed
    }

    pub fn register_event(&mut self, event: winit::event::KeyEvent) {
        match event.logical_key {
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Control) => {
                if event.state == ElementState::Pressed {
                    self.control_button = ElementState::Pressed;
                } else {
                    self.control_button = ElementState::Released;
                }
            }
            _ => (),
        }
    }
}
