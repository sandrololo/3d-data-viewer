use glam::{Mat4, Vec3, Vec4};
use tracing::trace;
use winit::{dpi::PhysicalPosition, event::ElementState};

struct MousePosition(Vec3);

pub struct Mouse {
    left_button: ElementState,
    last_position: MousePosition,
    last_transformation: Mat4,
}

impl Default for Mouse {
    fn default() -> Self {
        Self::new()
    }
}

impl Mouse {
    pub fn new() -> Self {
        Self {
            left_button: ElementState::Released,
            last_position: MousePosition(Vec3::new(0.0, 0.0, 1.0)),
            last_transformation: mat4_from_rotation_axis(Vec3::new(1.0, 0.0, 0.0), 30.0)
                * mat4_from_rotation_axis(Vec3::new(0.0, 1.0, 0.0), 30.0),
        }
    }

    pub fn get_current_transformation(&self) -> Mat4 {
        self.last_transformation
    }

    pub fn cursor_moved(&mut self, new_position: PhysicalPosition<f64>) {
        trace!("Cursor moved to position: {:?}", new_position);
        if ElementState::Pressed == self.left_button {
            let new_position = physical_position_to_vec3(new_position);
            let rot_axis = self.last_position.0.cross(new_position.0);
            let axis_len = rot_axis.length();
            let rot = mat4_from_rotation_axis(rot_axis, axis_len * 180.0 / std::f32::consts::PI);
            let transformation = rot * self.last_transformation;
            self.last_transformation = transformation;
            self.last_position = new_position;
        }
    }

    pub fn mouse_down(&mut self) {
        trace!("Mouse button down");
        self.left_button = ElementState::Pressed;
    }

    pub fn mouse_up(&mut self) {
        trace!("Mouse button up");
        self.left_button = ElementState::Released;
    }
}

fn physical_position_to_vec3(pos: PhysicalPosition<f64>) -> MousePosition {
    MousePosition(Vec3::new(pos.x as f32, pos.y as f32, 1.0))
}

fn mat4_from_rotation_axis(axs: Vec3, phi: f32) -> Mat4 {
    let a = Vec3::normalize(axs);
    let t = phi * std::f32::consts::PI / 180.0;
    let c = f32::cos(t);
    let s = f32::sin(t);
    let d = 1.0 - c;

    let d00 = d * a[0] * a[0];
    let d01 = d * a[0] * a[1];
    let d02 = d * a[0] * a[2];
    let d11 = d * a[1] * a[1];
    let d12 = d * a[1] * a[2];
    let d22 = d * a[2] * a[2];

    let s0 = s * a[0];
    let s1 = s * a[1];
    let s2 = s * a[2];

    Mat4 {
        x_axis: Vec4::new(d00 + c, d01 - s2, d02 + s1, 0.0),
        y_axis: Vec4::new(d01 + s2, d11 + c, d12 - s0, 0.0),
        z_axis: Vec4::new(d02 - s1, d12 + s0, d22 + c, 0.0),
        w_axis: Vec4::new(0.0, 0.0, 0.0, 1.0),
    }
}
