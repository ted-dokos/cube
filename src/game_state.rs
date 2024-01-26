use std::time::Instant;

use cgmath::Vector3;

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
                (0.0, 4.0, 8.0).into(),
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
        const LATERAL_ACCEL: Vector3<f32> = Vector3::<f32>::new(ACCEL, 0.0, 0.0);
        let delta_t = (*TIME_PER_GAME_TICK).as_secs_f32();
        let mut delta_v: Vector3<f32> = (0.0, 0.0, 0.0).into();
        let camera_vel = self.camera.get_velocity();
        if input.right && !input.left {
            delta_v += delta_t * LATERAL_ACCEL;
            //self.camera.move_eye(&lateral_velocity, (*TIME_PER_GAME_TICK).as_secs_f32());
        } else if input.left && !input.right {
            delta_v -= delta_t * LATERAL_ACCEL;
            //self.camera.move_eye(&-lateral_velocity, (*TIME_PER_GAME_TICK).as_secs_f32());
        } else {
            // Neither or both are pressed, apply lateral damping.
            delta_v += (-delta_t * camera_vel.x, 0.0, 0.0).into();
            //self.camera.move_eye(&delta_v, (*TIME_PER_GAME_TICK).as_secs_f32());
        }
        const FWD_ACCEL: Vector3<f32> = Vector3::<f32>::new(0.0, 0.0, -ACCEL);
        if input.forward && !input.backward {
            delta_v += delta_t * FWD_ACCEL;
        } else if input.backward && !input.forward {
            delta_v -= delta_t * FWD_ACCEL;
        } else {
            // Neither or both are pressed, apply forward damping.
            delta_v += (0.0, 0.0, -delta_t * camera_vel.z).into();
        }
        self.camera.move_eye(&delta_v, delta_t);
    }
}

pub struct InputState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
}

impl InputState {
    pub fn new() -> Self {
        InputState { forward: false, backward: false, left: false, right: false }
    }
}
