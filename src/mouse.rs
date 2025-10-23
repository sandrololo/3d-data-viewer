use glam::{Mat4, Vec3, Vec4};
use tracing::error;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::ElementState,
};

struct MousePosition(Vec3);

pub struct Mouse {
    current_position: PhysicalPosition<f64>,
    left_button: ElementState,
    control_button: ElementState,
    initial_trans_position: MousePosition,
    initial_transformation: Mat4,
    current_transformation: Mat4,
    window_size: PhysicalSize<u32>,
}

impl Default for Mouse {
    fn default() -> Self {
        Self::new()
    }
}

impl Mouse {
    pub fn new() -> Self {
        let default_trans = mat4_from_rotation_axis(Vec3::new(0.0, 1.0, 0.0), 45.0)
            * mat4_from_rotation_axis(Vec3::new(1.0, 0.0, 0.0), 240.0);
        Self {
            current_position: PhysicalPosition::new(0.0, 0.0),
            left_button: ElementState::Released,
            control_button: ElementState::Released,
            initial_trans_position: MousePosition(Vec3::new(0.5, 0.5, 1.0)),
            initial_transformation: default_trans,
            current_transformation: default_trans,
            window_size: PhysicalSize::new(100, 100),
        }
    }

    pub fn get_current_transformation(&self) -> Mat4 {
        self.current_transformation
    }

    pub fn cursor_moved(&mut self, new_position: PhysicalPosition<f64>) -> anyhow::Result<()> {
        self.current_position = new_position;
        if ElementState::Pressed == self.left_button {
            let new_position = self.physical_position_to_vec3(new_position)?;
            if !self.pointer_inside(new_position.0) {
                return Ok(());
            }
            if ElementState::Pressed == self.control_button {
                let trans = mat4_from_translation(new_position.0 - self.initial_trans_position.0);
                self.current_transformation = trans * self.initial_transformation;
            } else {
                let rot_axis = -self.initial_trans_position.0.cross(new_position.0);
                let axis_len = rot_axis.length();
                let rot = mat4_from_rotation_axis(rot_axis, axis_len * 100.0);
                self.current_transformation = rot * self.initial_transformation;
            }
        }
        Ok(())
    }

    fn pointer_inside(&self, pos: Vec3) -> bool {
        pos.x >= -1.0 && pos.x <= 1.0 && pos.y >= -1.0 && pos.y <= 1.0
    }

    pub fn mouse_down(&mut self) {
        self.left_button = ElementState::Pressed;
        match self.physical_position_to_vec3(self.current_position) {
            Ok(pos) => self.initial_trans_position = pos,
            Err(e) => error!("Failed to calculate pointer position: {}", e),
        }
        self.initial_transformation = self.current_transformation;
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
