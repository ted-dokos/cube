use std::time::{Duration, Instant};

use cgmath::{Vector3, Zero};

use crate::camera::Camera;

const GAME_TICKS_PER_SECOND: f64 = 25.0;

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
                (0.0, 1.0, 2.0).into(),
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
        let TIME_PER_GAME_TICK = Duration::from_secs_f64(1.0 / GAME_TICKS_PER_SECOND);
        self.tick += 1;
        self.update_instant = step_time;
        const LATERAL_ACCEL: Vector3<f32> = Vector3::<f32>::new(1.0, 0.0, 0.0);
        let lateral_velocity: Vector3<f32> = (TIME_PER_GAME_TICK.as_secs_f32()) * LATERAL_ACCEL;
        if input.right && !input.left {
            self.camera.move_eye(&lateral_velocity, TIME_PER_GAME_TICK.as_secs_f32());
        } else if input.left {
            self.camera.move_eye(&-lateral_velocity, TIME_PER_GAME_TICK.as_secs_f32());
        } else {
            // neither are pressed
            self.camera.move_eye(&Vector3::<f32>::zero(), TIME_PER_GAME_TICK.as_secs_f32());
        }
    }
}

pub struct InputState {
    pub left: bool,
    pub right: bool,
}

impl InputState {
    pub fn new() -> Self {
        InputState { left: false, right: false }
    }
}
