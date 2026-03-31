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

    pub(crate) fn update_modifiers(&mut self, modifiers: winit::event::Modifiers) {
        self.control_button = if modifiers.state().control_key() {
            ElementState::Pressed
        } else {
            ElementState::Released
        };
    }

    pub(crate) fn register_event(&mut self, event: winit::event::KeyEvent) {
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
