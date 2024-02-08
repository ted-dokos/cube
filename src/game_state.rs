use std::time::Instant;

use cgmath::{num_traits::abs, InnerSpace, Vector3, Zero};

use crate::{
    camera::Camera,
    constants::{GRAVITY, PLAYER_FORCE, TIME_PER_GAME_TICK},
    gpu_state::InstanceRaw,
    physics::{Collision, Physics},
    rotor::Rotor,
};

#[derive(Clone)]
pub struct ModelWithInstances {
    pub id: u32,
    pub instances: Vec<Instance>,
}

#[derive(Clone)]
struct Player {
    camera: Camera,
    physics: Physics,
}

const CAMERA_PHYSICS_OFFSET: f32 = 0.4;

#[derive(Clone)]
pub struct GameState {
    player: Player,
    tick: isize,
    update_instant: Instant,
    pub instanced_entities: Vec<ModelWithInstances>,
    // pub cube_instances: Vec<Instance>,
    //pub simple_cube_instances: Vec<Instance>,
}
impl GameState {
    pub fn new(aspect_ratio: f32) -> Self {
        let mut player_physics = Physics::new();
        player_physics.collision = Collision::new(
            [
                Vector3::new(0.125, 0.125, 0.5),
                Vector3::new(-0.125, 0.125, 0.5),
                Vector3::new(0.125, -0.125, 0.5),
                Vector3::new(-0.125, -0.125, 0.5),
                Vector3::new(0.125, 0.125, -0.5),
                Vector3::new(-0.125, 0.125, -0.5),
                Vector3::new(0.125, -0.125, -0.5),
                Vector3::new(-0.125, -0.125, -0.5),
            ]
            .into(),
            [].into(),
        );
        let mut instanced_entities = Vec::<ModelWithInstances>::new();
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
                        Rotor::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
                    } else {
                        Rotor::from_axis_angle(
                            position.normalize(),
                            //cgmath::Deg(0.0),
                            // cgmath::Deg(3.0 * ((x + 1) * (z + 1)) as f32),
                            cgmath::Deg(45.0),
                        )
                    };
                    Instance { position, scale: 1.0, rotation, shader: Shader::Texture }
                })
            })
            .collect::<Vec<_>>();
        // Big floor instance.
        instances.push(Instance {
            position: (0.0, -20.0, 0.0).into(),
            scale: 11.0,
            rotation: Rotor::identity(),
            shader: Shader::Texture,
        });
        // Light instance.
        instances.push(Instance {
            position: Vector3::new(2.0, 2.0, 2.0),
            scale: 0.25,
            rotation: Rotor::identity(),
            shader: Shader::NonMaterial,
        });
        instanced_entities.push(ModelWithInstances { id: 0, instances });
        let simple_cube_instances = vec![
            Instance {
                position: (0.0, -4.5, 0.0).into(),
                scale: 0.5,
                rotation: Rotor::identity(),
                shader: Shader::Pulse,
            },
            Instance {
                position: (3.0, -4.5, 0.0).into(),
                scale: 0.5,
                rotation: Rotor::identity(),
                shader: Shader::Ripple,
            },
            Instance {
                position: (-3.0, -4.5, 0.0).into(),
                scale: 0.5,
                rotation: Rotor::identity(),
                shader: Shader::ColorTween,
            },
            // Interesting "bug": the spheres don't show up through this cube, because they are
            // drawn later in the scene. See
            // https://docs.godotengine.org/en/latest/tutorials/3d/3d_rendering_limitations.html#transparency-sorting.
            Instance {
                position: (-6.0, -4.5, 0.0).into(),
                scale: 0.5,
                rotation: Rotor::identity(),
                shader: Shader::SimpleTransparency,
            },
            Instance {
                position: (3.0, -4.5, 3.0).into(),
                scale: 0.75,
                rotation: Rotor::identity(),
                shader: Shader::Aerogel,
            }
        ];
        instanced_entities.push(ModelWithInstances { id: 1, instances: simple_cube_instances });
        instanced_entities.push(ModelWithInstances {
            id: 2,
            instances: vec![Instance {
                position: (-3.0, -4.5, 3.0).into(),
                scale: 0.5,
                rotation: Rotor::identity(),
                shader: Shader::Pulse,
            }],
        });
        instanced_entities.push(ModelWithInstances {
            id: 3,
            instances: vec![Instance {
                position: (-3.0, -4.5, 6.0).into(),
                scale: 0.5,
                rotation: Rotor::identity(),
                shader: Shader::ColorTween,
            }],
        });
        instanced_entities.push(ModelWithInstances {
            id: 4,
            instances: vec![Instance {
                position: (-6.0, -4.5, -3.0).into(),
                scale: 0.5,
                rotation: Rotor::identity(),
                shader: Shader::SimpleTransparency,
            }],
        });

        const CAMERA_EYE_Y: f32 = 5.0;
        player_physics.position = (0.0, CAMERA_EYE_Y - CAMERA_PHYSICS_OFFSET, 10.0).into();
        GameState {
            player: Player {
                camera: Camera::new(
                    // position the camera 1 unit up and 2 units back
                    // +z is out of the screen
                    (0.0, CAMERA_EYE_Y, 10.0).into(),
                    // have it look at the origin
                    (0.0, -1.0, -2.0).into(),
                    // which way is "up"
                    Vector3::unit_y(),
                    aspect_ratio,
                    45.0,
                    0.1,
                    100.0,
                ),
                physics: player_physics,
            },
            tick: 0,
            update_instant: Instant::now(),
            // cube_instances: instances,
            //simple_cube_instances,
            instanced_entities,
        }
    }
    pub fn change_camera_aspect(&mut self, aspect_ratio: f32) {
        self.player.camera.set_aspect(aspect_ratio);
    }
    pub fn get_camera(&self) -> Camera {
        self.player.camera
    }
    pub fn update(&mut self, input: &InputState, step_time: Instant) {
        self.tick += 1;
        self.update_instant = step_time;
        self.player.physics.accel = (0.0, GRAVITY, 0.0).into();
        let lateral_force = PLAYER_FORCE
            * cgmath::Vector3::normalize(
                [-self.player.camera.direction.z, 0.0, self.player.camera.direction.x].into(),
            );

        let delta_t = (*TIME_PER_GAME_TICK).as_secs_f32();
        if input.right && !input.left {
            self.player.physics.apply_force(lateral_force);
        } else if input.left && !input.right {
            self.player.physics.apply_force(-lateral_force);
        } else {
            // Neither or both are pressed, apply lateral damping.
            self.player.physics.apply_force(
                -Vector3::dot(
                    self.player.physics.mass * self.player.physics.velocity,
                    lateral_force / PLAYER_FORCE,
                ) * (lateral_force / PLAYER_FORCE)
                    * (1.0 / (10.0 * delta_t)),
            );
        }
        let fwd_force = PLAYER_FORCE
            * cgmath::Vector3::normalize(
                [self.player.camera.direction.x, 0.0, self.player.camera.direction.z].into(),
            );

        // TODO: this acts terribly if you quickly switch between left/right or fwd/back
        if input.forward && !input.backward {
            self.player.physics.apply_force(fwd_force);
        } else if input.backward && !input.forward {
            self.player.physics.apply_force(-fwd_force);
        } else {
            // Neither or both are pressed, apply forward damping.
            self.player.physics.apply_force(
                -Vector3::dot(
                    self.player.physics.mass * self.player.physics.velocity,
                    fwd_force / PLAYER_FORCE,
                ) * (fwd_force / PLAYER_FORCE)
                    * (1.0 / (10.0 * delta_t)),
            );
        }
        if input.jump && self.player.physics.position.y <= -4.999 {
            self.player.physics.velocity += (0.0, 5.0, 0.0).into();
        }
        let delta_pos = self.player.physics.update(delta_t, 10.0);
        self.player.camera.eye += delta_pos;
        if self.player.physics.position.y < -5.0 {
            self.player.physics.position.y = -5.0;
            self.player.physics.velocity.y = 0.0;
            // TODO: clearly the player update code should be responsible for moving the eye /
            // center-of-mass in tandem.
            self.player.camera.eye =
                self.player.physics.position + Vector3::new(0.0, CAMERA_PHYSICS_OFFSET, 0.0);
        }

        const ROTATION_MOVEMENT_DEG: f32 = 0.1;
        let lateral_rot = Rotor::from_axis_angle(
            cgmath::Vector3::unit_y(),
            cgmath::Deg(-ROTATION_MOVEMENT_DEG * input.mouse_x as f32),
        );
        let vertical_rot = Rotor::from_axis_angle(
            cgmath::Vector3::normalize(
                [self.player.camera.direction.z, 0.0, -self.player.camera.direction.x].into(),
            ),
            cgmath::Deg(ROTATION_MOVEMENT_DEG * input.mouse_y as f32),
        );
        // Prevent the camera from getting too close to a vertical pole, while still allowing for
        // lateral movement.
        const POLAR_THRESHOLD: f32 = 0.001;
        let new_vertical =
            cgmath::Vector3::normalize(vertical_rot.rotate_vector(self.player.camera.direction));
        if abs(cgmath::Vector3::dot(new_vertical, cgmath::Vector3::unit_y()))
            > 1.0 - POLAR_THRESHOLD
        {
            self.player.camera.direction =
                cgmath::Vector3::normalize(lateral_rot.rotate_vector(self.player.camera.direction));
        } else {
            self.player.camera.direction =
                cgmath::Vector3::normalize(lateral_rot.rotate_vector(new_vertical));
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
    pub jump: bool,
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
            jump: false,
        }
    }
    pub fn post_update_reset(&mut self) {
        self.mouse_x = 0;
        self.mouse_y = 0;
        self.jump = false;
    }
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum Shader {
    Texture = 0,
    NonMaterial = 1,
    Pulse = 2,
    Ripple = 3,
    ColorTween = 4,
    SimpleTransparency = 5,
    Aerogel = 6,
}

#[derive(Clone, Copy)]
pub struct Instance {
    pub position: cgmath::Vector3<f32>,
    pub scale: f32,
    pub rotation: Rotor,
    pub shader: Shader,
}
impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            pos: self.position.into(),
            scale: self.scale,
            rot: self.rotation.into(),
            shader: self.shader as u32,
        }
    }
}
