struct Camera {
    view_pos: vec3<f32>,
    view_proj: mat4x4<f32>,
};
@group(1) @binding(0)
var<uniform> camera: Camera;

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}
@group(2) @binding(0)
var<uniform> light: Light;

struct Time {
    secs: f32,
}
@group(3) @binding(0)
var<uniform> time: Time;

// Vertex shader
struct InstanceInput {
    @location(5) position: vec3<f32>,
    @location(6) scale: f32,
    @location(7) rotation: vec4<f32>,
    @location(8) shader: u32,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
}
struct NonmaterialVertexInput {
    @location(0) position: vec3<f32>,
}

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
    var s_x: f32 = rotor.x * vector.x + rotor.y * vector.y + rotor.z * vector.z;
    var s_y: f32 = rotor.x * vector.y - rotor.y * vector.x + rotor.w * vector.z;
    var s_z: f32 = rotor.x * vector.z - rotor.w * vector.y - rotor.z * vector.x;
    var s_xyz: f32 = rotor.y * vector.z + rotor.w * vector.x - rotor.z * vector.y;

    // Now calculate SR':
    var out: vec3<f32>;
    out.x = s_x * rotor.x + s_y * rotor.y + s_xyz * rotor.w + s_z * rotor.z;
    out.y = s_y * rotor.x - s_x * rotor.y + s_z * rotor.w - s_xyz * rotor.z;
    out.z = s_z * rotor.x + s_xyz * rotor.y - s_y * rotor.w + s_x * rotor.z;
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
    out.world_position = calculate_world_position(instance.scale * model.position, instance);
    out.clip_position = calculate_clip_position(out.world_position);
    out.shader = instance.shader;
    return out;
}
@vertex
fn nonmaterial_vs_main(
    model: NonmaterialVertexInput,
    instance: InstanceInput
) -> NonmaterialFragmentInput {
    var out: NonmaterialFragmentInput;
    out.clip_position = calculate_clip_position(
        calculate_world_position(instance.scale * model.position + light.position, instance)
    );
    return out;
}

// Fragment shader
struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
    @location(3) shader: u32,
};
struct LightingOutput {
    ambient_color: vec3<f32>,
    diffuse_color: vec3<f32>,
    specular_color: vec3<f32>,
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

fn calculate_lighting(in: FragmentInput) -> LightingOutput {
    var out: LightingOutput;
    let ambient_strength = 0.2;
    out.ambient_color = light.color * ambient_strength;

    let light_dir = normalize(light.position - in.world_position);
    let diffuse_strength = max(dot(in.world_normal, light_dir), 0.0);
    out.diffuse_color = light.color * diffuse_strength;

    let view_dir = normalize(camera.view_pos - in.world_position);
    let half_dir = normalize(view_dir + light_dir);
    let specular_strength = pow(max(dot(in.world_normal, half_dir), 0.0), 32.0);
    out.specular_color = light.color * specular_strength;

    return out;
}

// Enums for the type of shader.
const Pulse = 2u;
const Ripple = 3u;
const ColorTween = 4u;
const SimpleTransparency = 5u;
@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    var unlit: vec4<f32>;
    switch in.shader {
        case Pulse: { unlit = fs_pulse(in); }
        case Ripple: { unlit = fs_ripple(in); }
        case ColorTween: { unlit = fs_color_tween(in); }
        case SimpleTransparency: { unlit = vec4<f32>(0.5); }
        default: { unlit = vec4<f32>(0.0, 0.0, 0.0, 1.0); }
    }
    let light = calculate_lighting(in);
    let result = (light.ambient_color + light.diffuse_color + light.specular_color) * unlit.xyz;
    return vec4<f32>(result, unlit.a);
}
fn fs_pulse(in: FragmentInput) -> vec4<f32> {
    var object_color: vec4<f32> = vec4<f32>(0.03, 0.03, 0.03, 1.0);
    object_color.x += 0.9 * (cos(time.secs * 2.0) + 1.0) / 2.0;
    return object_color;
}
fn fs_ripple(in: FragmentInput) -> vec4<f32> {
    let uv = in.tex_coords;
    let radius = length(uv);
    let color_str = pow((cos(radius * 20.0 - 4.0 * time.secs) + 1.0) / 2.0, 2.0);
    return vec4<f32>(color_str, color_str, color_str, 1.0);
}
const NumTweenColors = 6;
const TweenTimeSecs = 3.0;
fn fs_color_tween(in: FragmentInput) -> vec4<f32> {
    var TweenColors = array<vec3<f32>, NumTweenColors>(
        vec3<f32>(1.0, 0.0, 0.0), // red
        vec3<f32>(1.0, 1.0, 0.0), // yellow
        vec3<f32>(0.0, 1.0, 0.0), // green
        vec3<f32>(0.0, 1.0, 1.0), // cyan
        vec3<f32>(0.0, 0.0, 1.0), // blue
        vec3<f32>(1.0, 0.0, 1.0), // purple
    );
    let split = modf(time.secs / TweenTimeSecs);
    let prev_idx = i32(split.whole) % NumTweenColors;
    let next_idx = (prev_idx + 1) % NumTweenColors;
    return vec4<f32>(split.fract * TweenColors[next_idx] + (1.0 - split.fract) * TweenColors[prev_idx], 1.0);
}