use std::time::Instant;

use cgmath::{num_traits::abs, InnerSpace, Rotation, Rotation3, Vector3};

use crate::{camera::Camera, constants::TIME_PER_GAME_TICK};

#[derive(Clone, Copy)]
pub struct GameState {
    camera: Camera,
    tick: isize,
    update_instant: Instant,
}

impl GameState {
    pub fn new(aspect_ratio: f32) -> Self {
        GameState {
            camera: Camera::new(
                // position the camera 1 unit up and 2 units back
                // +z is out of the screen
                (0.0, 5.0, 10.0).into(),
                // have it look at the origin
                (0.0, -1.0, -2.0).into(),
                // which way is "up"
                Vector3::unit_y(),
                aspect_ratio,
                45.0,
                0.1,
                100.0,
            ),
            tick: 0,
            update_instant: Instant::now(),
        }
    }
    pub fn change_camera_aspect(&mut self, aspect_ratio: f32) {
        self.camera.set_aspect(aspect_ratio);
    }
    pub fn get_camera(&self) -> Camera {
        self.camera
    }
    pub fn update(&mut self, input: &InputState, step_time: Instant) {
        self.tick += 1;
        self.update_instant = step_time;
        const ACCEL: f32 = 3.0;
        let lateral_accel = ACCEL * cgmath::Vector3::normalize([-self.camera.direction.z, 0.0, self.camera.direction.x].into());
        let delta_t = (*TIME_PER_GAME_TICK).as_secs_f32();
        let mut delta_v: Vector3<f32> = (0.0, 0.0, 0.0).into();
        let camera_vel = self.camera.get_velocity();
        if input.right && !input.left {
            delta_v += delta_t * lateral_accel;
        } else if input.left && !input.right {
            delta_v -= delta_t * lateral_accel;
        } else {
            // Neither or both are pressed, apply lateral damping.
            delta_v += (-delta_t * camera_vel.x, 0.0, 0.0).into();
        }
        let fwd_accel = ACCEL * cgmath::Vector3::normalize([self.camera.direction.x, 0.0, self.camera.direction.z].into());
        if input.forward && !input.backward {
            delta_v += delta_t * fwd_accel;
        } else if input.backward && !input.forward {
            delta_v -= delta_t * fwd_accel;
        } else {
            // Neither or both are pressed, apply forward damping.
            delta_v += (0.0, 0.0, -delta_t * camera_vel.z).into();
        }
        self.camera.move_eye(&delta_v, delta_t);
        const ROTATION_MOVEMENT_DEG: f32 = 0.1;
        let lateral_rot = cgmath::Quaternion::from_axis_angle(
            cgmath::Vector3::unit_y(),
            cgmath::Deg(-ROTATION_MOVEMENT_DEG * input.mouse_x as f32),
        );
        let vertical_rot = cgmath::Quaternion::from_axis_angle(
            cgmath::Vector3::normalize(
                [self.camera.direction.z, 0.0, -self.camera.direction.x].into(),
            ),
            cgmath::Deg(ROTATION_MOVEMENT_DEG * input.mouse_y as f32),
        );
        const CAMERA_PRECISION_EPS: f32 = 0.001;
        let new_camera_direction = cgmath::Vector3::normalize(
            lateral_rot.rotate_vector(vertical_rot.rotate_vector(self.camera.direction)),
        );
        if !(abs(cgmath::Vector3::dot(new_camera_direction, cgmath::Vector3::unit_y()))
            > 1.0 - CAMERA_PRECISION_EPS)
        {
            self.camera.direction = new_camera_direction;
        } else {
            self.camera.direction =
                cgmath::Vector3::normalize(lateral_rot.rotate_vector(self.camera.direction));
        }
    }
}

pub struct InputState {
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
}

impl InputState {
    pub fn new() -> Self {
        InputState {
            mouse_x: 0,
            mouse_y: 0,
            forward: false,
            backward: false,
            left: false,
            right: false,
        }
    }
    pub fn post_update_reset(&mut self) {
        self.mouse_x = 0;
        self.mouse_y = 0;
    }
}
