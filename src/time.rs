#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TimeUniform {
    time_secs: f32,
    _padding: [f32; 3],
}
impl TimeUniform {
    pub fn new(time_secs: f32) -> Self {
        TimeUniform {
            time_secs,
            _padding: [0.0, 0.0, 0.0],
        }
    }
}