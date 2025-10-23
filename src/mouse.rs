use glam::{Mat4, Vec3, Vec4};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::ElementState,
};

struct MousePosition(Vec3);

pub struct Mouse {
    left_button: ElementState,
    control_button: ElementState,
    last_position: MousePosition,
    last_transformation: Mat4,
    window_size: PhysicalSize<u32>,
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
            control_button: ElementState::Released,
            last_position: MousePosition(Vec3::new(0.5, 0.5, 1.0)),
            last_transformation: mat4_from_rotation_axis(Vec3::new(1.0, 0.0, 0.0), 30.0)
                * mat4_from_rotation_axis(Vec3::new(0.0, 1.0, 0.0), 30.0),
            window_size: PhysicalSize::new(100, 100),
        }
    }

    pub fn get_current_transformation(&self) -> Mat4 {
        self.last_transformation
    }

    pub fn cursor_moved(&mut self, new_position: PhysicalPosition<f64>) -> anyhow::Result<()> {
        if ElementState::Pressed == self.left_button {
            let new_position = self.physical_position_to_vec3(new_position)?;
            if ElementState::Pressed == self.control_button {
                let trans = mat4_from_translation(
                    (new_position.0 - self.last_position.0) * Vec3::new(0.5, 0.5, 0.0),
                );
                self.last_transformation = trans * self.last_transformation;
            } else {
                let rot_axis = self.last_position.0.cross(new_position.0);
                let axis_len = rot_axis.length();
                let rot = mat4_from_rotation_axis(rot_axis, axis_len * 100.0);
                self.last_transformation = rot * self.last_transformation;
            }
            self.last_position = new_position;
        }
        Ok(())
    }

    pub fn mouse_down(&mut self) {
        self.left_button = ElementState::Pressed;
    }

    pub fn mouse_up(&mut self) {
        self.left_button = ElementState::Released;
    }

    pub fn control_down(&mut self) {
        self.control_button = ElementState::Pressed;
    }

    pub fn control_up(&mut self) {
        self.control_button = ElementState::Released;
    }

    pub fn update_window_size(&mut self, size: PhysicalSize<u32>) {
        self.window_size = size;
    }

    fn physical_position_to_vec3(
        &self,
        pos: PhysicalPosition<f64>,
    ) -> anyhow::Result<MousePosition> {
        let w = f64::try_from(self.window_size.width - 1)?;
        let h = f64::try_from(self.window_size.height - 1)?;
        let x = (2.0 * pos.x / w - 1.0) as f32;
        let y = (1.0 - 2.0 * pos.y / h) as f32;
        Ok(MousePosition(Vec3::new(x, y, 1.0)))
    }
}

fn mat4_from_translation(v: Vec3) -> Mat4 {
    Mat4 {
        x_axis: Vec4::new(1.0, 0.0, 0.0, 0.0),
        y_axis: Vec4::new(0.0, 1.0, 0.0, 0.0),
        z_axis: Vec4::new(0.0, 0.0, 1.0, 0.0),
        w_axis: Vec4::new(v[0], v[1], v[2], 1.0),
    }
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
