use cgmath::{Deg, Matrix4, Point3, Vector3};

#[derive(Clone, Copy)]
pub struct Camera {
    eye: Point3<f32>, // position of the camera
    velocity: Vector3<f32>,
    pub direction: Vector3<f32>,
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
        Camera { eye, velocity: (0.0, 0.0, 0.0).into(), direction, up, aspect, fovy, znear, zfar }
    }
    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let view = cgmath::Matrix4::look_to_rh(self.eye, self.direction, self.up);
        let proj = cgmath::perspective(Deg(self.fovy), self.aspect, self.znear, self.zfar);

        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
    pub fn move_eye(&mut self, delta_v: &Vector3<f32>, delta_t: f32) {
        self.velocity += *delta_v;
        self.eye += 0.5 * delta_t * self.velocity;
    }
    // pub fn get_eye(&self) -> Point3<f32> {
    //     self.eye
    // }
    pub fn get_velocity(&self) -> Vector3<f32> {
        self.velocity
    }
    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_position: [f32; 3],
    _padding: f32,
    // We can't use cgmath with bytemuck directly, so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    pub view_proj: [[f32; 4]; 4],
}
impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_position: [0.0; 3],
            _padding: 0.0,
            view_proj: cgmath::Matrix4::identity().into() }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_position = camera.eye.into();
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}