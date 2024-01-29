use std::time::Instant;

use cgmath::{num_traits::abs, InnerSpace, Rotation, Rotation3, Vector3, Zero};

use crate::{camera::Camera, constants::{GRAVITY, TIME_PER_GAME_TICK}, gpu_state::InstanceRaw};

#[derive(Clone)]
pub struct GameState {
    camera: Camera,
    tick: isize,
    update_instant: Instant,
    pub cube_instances: Vec<Instance>,
}

impl GameState {
    pub fn new(aspect_ratio: f32) -> Self {
        const NUM_INSTANCES_PER_ROW: u32 = 10;
        const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(
            NUM_INSTANCES_PER_ROW as f32 * 0.5,
            0.0,
            NUM_INSTANCES_PER_ROW as f32 * 0.5,
        );
        const SPACE_BETWEEN: f32 = 3.0;
        let mut instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let position = (-SPACE_BETWEEN)
                        * (cgmath::Vector3 { x: x as f32, y: 0.0, z: z as f32 }
                            - INSTANCE_DISPLACEMENT);

                    let rotation = if position.is_zero() {
                        // this is needed so an object at (0, 0, 0) won't get scaled to zero
                        // as Quaternions can affect scale if they're not created correctly
                        cgmath::Quaternion::from_axis_angle(
                            cgmath::Vector3::unit_z(),
                            cgmath::Deg(0.0),
                        )
                    } else {
                        cgmath::Quaternion::from_axis_angle(
                            position.normalize(),
                            //cgmath::Deg(0.0),
                            // cgmath::Deg(3.0 * ((x + 1) * (z + 1)) as f32),
                            cgmath::Deg(45.0),
                        )
                    };
                    Instance { position, scale: 1.0, rotation }
                })
            })
            .collect::<Vec<_>>();
        instances.push(Instance {
            position: (0.0, -20.0, 0.0).into(),
            scale: 11.0,
            rotation: cgmath::Quaternion::<f32>::new(1.0, 0.0, 0.0, 0.0),
        });
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
            cube_instances: instances,
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
        delta_v += (0.0, delta_t * GRAVITY, 0.0).into();
        self.camera.move_eye(&delta_v, delta_t);
        if self.camera.eye.y < -5.0 {
            self.camera.eye.y = -5.0;
            self.camera.velocity.y = 0.0;
        }
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
        // Prevent the camera from getting too close to a vertical pole, while still allowing for lateral movement.
        const POLAR_THRESHOLD: f32 = 0.001;
        let new_vertical = cgmath::Vector3::normalize(vertical_rot.rotate_vector(self.camera.direction)
        );
        if abs(cgmath::Vector3::dot(new_vertical, cgmath::Vector3::unit_y()))
            > 1.0 - POLAR_THRESHOLD
        {
            self.camera.direction =
                cgmath::Vector3::normalize(lateral_rot.rotate_vector(self.camera.direction));
        } else {
            self.camera.direction = cgmath::Vector3::normalize(lateral_rot.rotate_vector(new_vertical));

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

#[derive(Clone, Copy)]
pub struct Instance {
    pub position: cgmath::Vector3<f32>,
    pub scale: f32,
    pub rotation: cgmath::Quaternion<f32>,
}
impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: [
                self.position.x,
                self.position.y,
                self.position.z,
                self.scale,
                self.rotation.s,
                -self.rotation.v.z,
                -self.rotation.v.x,
                -self.rotation.v.y,
            ],
        }
    }
}