use cgmath::{Deg, Matrix4, Point3, Vector3};

pub struct Camera {
    eye: Point3<f32>,
    direction: Vector3<f32>,
    up: Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    pub fn new(
        eye: Point3<f32>,
        direction: Vector3<f32>,
        up: Vector3<f32>,
        aspect: f32,
        fovy: f32,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Camera {
            eye,
            direction,
            up,
            aspect,
            fovy,
            znear,
            zfar,
        }
    }
    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let view = cgmath::Matrix4::look_to_rh(self.eye, self.direction, self.up);
        let proj = cgmath::perspective(Deg(self.fovy), self.aspect, self.znear, self.zfar);

        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
    pub fn move_eye(&mut self, delta: &Vector3<f32>) {
        self.eye = self.eye + delta;
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);
