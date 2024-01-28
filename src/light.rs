#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
pub(crate) struct LightUniform {
    pub position: [f32; 3],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    pub(crate) _padding: u32,
    pub(crate) color: [f32; 3],
    pub(crate) _padding2: u32,
}

