struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}
@group(2) @binding(0)
var<uniform> light: Light;

// Vertex shader
struct InstanceInput {
    @location(5) position: vec3<f32>,
    @location(6) rotation: vec4<f32>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
}
struct NonmaterialVertexInput {
    @location(0) position: vec3<f32>,
}

struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
};

struct NonmaterialFragmentInput {
    @builtin(position) clip_position: vec4<f32>,
}

fn apply_rotor_to_vector(
    rotor: vec4<f32>,
    vector: vec3<f32>,
) -> vec3<f32> {
    // Assumption: rotor comes from a quaternion representing a rotation, and is therefore a unit
    // rotor.
    // Strategy: calculate RvR', where R' is "R-inverse" and is the conjugate of R.
    // Calculate S = Rv first:
    var s_x: f32 = rotor.x * vector.x + rotor.y * vector.y - rotor.w * vector.z;
    var s_y: f32 = rotor.x * vector.y - rotor.y * vector.x + rotor.z * vector.z;
    var s_z: f32 = rotor.x * vector.z - rotor.z * vector.y + rotor.w * vector.x;
    var s_xyz: f32 = rotor.y * vector.z + rotor.z * vector.x + rotor.w * vector.y;

    // Now calculate SR':
    var out: vec3<f32>;
    out.x = s_x * rotor.x + s_y * rotor.y + s_xyz * rotor.z - s_z * rotor.w;
    out.y = s_y * rotor.x - s_x * rotor.y + s_z * rotor.z + s_xyz * rotor.w;
    out.z = s_z * rotor.x + s_xyz * rotor.y - s_y * rotor.z + s_x * rotor.w;
    return out;
}

fn calculate_world_position(
    model_position: vec3<f32>,
    instance: InstanceInput,
) -> vec3<f32> {
    return apply_rotor_to_vector(instance.rotation, model_position) + instance.position;
}

fn calculate_clip_position(
    world_position: vec3<f32>
) -> vec4<f32> {
    return camera.view_proj * vec4<f32>(world_position, 1.0);
}
@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> FragmentInput {
    var out: FragmentInput;
    out.tex_coords = model.tex_coords;
    out.world_normal = apply_rotor_to_vector(instance.rotation, model.normal);
    out.world_position = calculate_world_position(model.position, instance);
    out.clip_position = calculate_clip_position(out.world_position);
    return out;
}
@vertex
fn nonmaterial_vs_main(
    model: NonmaterialVertexInput,
    instance: InstanceInput
) -> NonmaterialFragmentInput {
    let scale = 0.25;
    var out: NonmaterialFragmentInput;
    out.clip_position = calculate_clip_position(
        calculate_world_position(scale * model.position + light.position, instance));
    return out;
}

// Fragment shader
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;
@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let ambient_strength = 0.1;
    let ambient_color = light.color * ambient_strength;

    let light_dir = normalize(light.position - in.world_position);
    let diffuse_strength = max(dot(in.world_normal, light_dir), 0.0);
    let diffuse_color = light.color * diffuse_strength;

    let result = (ambient_color + diffuse_color) * object_color.xyz;
    return vec4<f32>(result, object_color.a);
}
@fragment
fn nonmaterial_fs_main(in: NonmaterialFragmentInput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}