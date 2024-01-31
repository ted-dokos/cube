use cgmath::{num_traits::abs, InnerSpace, Quaternion, Rad, Rotation3, Vector3};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Rotor {
    pub s: f32,
    pub xy: f32,
    pub xz: f32,
    pub yz: f32,
}
impl Rotor {
    #[allow(unused)]
    pub fn new(s: f32, xy: f32, xz: f32, yz: f32) -> Self {
        Rotor { s, xy, xz, yz }
    }
    /// Assumes that the input vector has already been normalized. Will not work correctly otherwise.
    pub fn from_axis_angle<A: Into<Rad<f32>>>(v: Vector3<f32>, angle: A) -> Rotor {
        debug_assert!(abs(v.magnitude() - 1.0) < 0.000001);
        // lol
        Rotor::from_quat(cgmath::Quaternion::from_axis_angle(v, angle))
    }
    pub fn from_quat(q: Quaternion<f32>) -> Rotor {
        Rotor { s: q.s, xy: -q.v.z, xz: q.v.y, yz: -q.v.x }
    }
    pub fn identity() -> Rotor {
        Rotor { s: 1.0, xy: 0.0, xz: 0.0, yz: 0.0 }
    }
    #[allow(unused)]
    pub fn inverse(&self) -> Rotor {
        Rotor { s: self.s, xy: -self.xy, xz: -self.xz, yz: -self.yz }
    }
    pub fn rotate_vector(&self, vec: Vector3<f32>) -> Vector3<f32> {
        // Calculate S = Rv
        let s_x = self.s * vec.x + self.xy * vec.y + self.xz * vec.z;
        let s_y = -self.xy * vec.x + self.s * vec.y + self.yz * vec.z;
        let s_z = -self.xz * vec.x - self.yz * vec.y + self.s * vec.z;
        let s_xyz = self.yz * vec.x - self.xz * vec.y + self.xy * vec.z;

        // Calculate SR' = RvR'
        Vector3 {
            x: s_x * self.s + s_y * self.xy + s_z * self.xz + s_xyz * self.yz,
            y: s_y * self.s - s_x * self.xy - s_xyz * self.xz + s_z * self.yz,
            z: s_z * self.s + s_xyz * self.xy - s_x * self.xz - s_y * self.yz,
        }
    }
}
