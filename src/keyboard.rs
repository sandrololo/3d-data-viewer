use winit::event::ElementState;

pub(crate) struct Keyboard {
    control_button: ElementState,
}

impl Default for Keyboard {
    fn default() -> Self {
        Self {
            control_button: ElementState::Released,
        }
    }
}

impl Keyboard {
    pub(crate) fn is_control_pressed(&self) -> bool {
        self.control_button == ElementState::Pressed
    }

    pub(crate) fn register_event(&mut self, event: winit::event::KeyEvent) {
        if let winit::keyboard::Key::Named(winit::keyboard::NamedKey::Control) = event.logical_key {
            self.control_button = event.state;
        }
    }
}
