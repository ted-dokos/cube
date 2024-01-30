use cgmath::{InnerSpace, One, Point3, Quaternion, Vector3, Zero};
use tobj::Model;

use crate::{
    constants::GRAVITY,
    model::{Mesh, ModelVertex},
};

#[derive(Clone)]
pub struct Physics {
    pub position: Point3<f32>,
    pub velocity: Vector3<f32>,
    pub accel: Vector3<f32>,
    pub mass: f32,
    pub angular_position: Quaternion<f32>,
    pub angular_velocity: Vector3<f32>,
    pub angular_accel: Vector3<f32>,
    pub collision: Collision,
}
impl Physics {
    pub fn new() -> Self {
        Physics {
            position: Point3::new(0.0, 0.0, 0.0),
            velocity: Vector3::zero(),
            accel: Vector3::new(0.0, GRAVITY, 0.0),
            mass: 1.0,
            angular_position: Quaternion::one(),
            angular_velocity: Vector3::zero(),
            angular_accel: Vector3::zero(),
            collision: Collision::new([].into(), [].into()),
        }
    }
    pub fn apply_force(&mut self, force: Vector3<f32>) {
        self.accel += force / self.mass;
    }
    // Returns the delta in position.
    pub fn update(&mut self, delta_t: f32, max_vel: f32) -> Vector3<f32> {
        let old_pos = self.position;
        let delta_v = self.accel * delta_t;
        let delta_pos = delta_t * (self.velocity + 0.5 * delta_v);
        self.velocity += delta_v;

        // TODO: this affects vertical velocity.
        if max_vel < 0.0 || self.velocity.magnitude() > max_vel {
            self.velocity = self.velocity.normalize_to(max_vel);
            self.position += self.velocity * delta_t;
        } else {
            self.position += delta_pos;
        }
        self.position - old_pos
    }
}

#[derive(Clone)]
pub struct Collision {
    pub vertices: Vec<Vector3<f32>>,
}
impl Collision {
    pub fn new(vertices: Vec<Vector3<f32>>, indices: Vec<u32>) -> Self {
        Collision { vertices }
    }
    fn calculate_bounding_box(vertices: &Vec<Vector3<f32>>) -> (Vector3<f32>, Vector3<f32>) {
        if vertices.is_empty() {
            return (Vector3::zero(), Vector3::zero());
        }
        let mut max: Vector3<f32> = vertices[0];
        let mut min: Vector3<f32> = vertices[0];
        for vertex in vertices {
            if vertex.x > max.x {
                max.x = vertex.x;
            }
            if vertex.y > max.y {
                max.x = vertex.y;
            }
            if vertex.z > max.z {
                max.x = vertex.z;
            }
            if vertex.x > min.x {
                min.x = vertex.x;
            }
            if vertex.y > min.y {
                min.x = vertex.y;
            }
            if vertex.z > min.z {
                min.x = vertex.z;
            }
        }
        (min, max)
    }
}
