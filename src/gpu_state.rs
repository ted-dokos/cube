/* WebGPUState: data and behavior needed to create and render using WebGPU. */
use crate::{
    camera::{Camera, CameraUniform},
    game_state::{GameState, Instance},
    light::LightUniform,
    model::{self, DescribeVB, Material, Mesh, ModelVertex},
    rotor::Rotor,
    texture,
    time::TimeUniform,
};

use cgmath::Rotation3;
use std::{
    ffi::c_void,
    mem::{self},
    result::Result,
};
use std::{ops::Range, time::Instant};
use wgpu::util::DeviceExt;
use windows::Win32::Foundation::POINT;
use windows::Win32::{
    Foundation::{HINSTANCE, HWND, RECT},
    UI::WindowsAndMessaging::*,
};

struct MyWindowHandle {
    win32handle: raw_window_handle::Win32WindowHandle,
}
impl MyWindowHandle {
    fn new(window: HWND, hinstance: HINSTANCE) -> Self {
        let mut h = raw_window_handle::Win32WindowHandle::empty();
        h.hwnd = window.0 as *mut c_void;
        h.hinstance = hinstance.0 as *mut c_void;
        Self { win32handle: h }
    }
}

unsafe impl raw_window_handle::HasRawWindowHandle for MyWindowHandle {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        raw_window_handle::RawWindowHandle::Win32(self.win32handle)
    }
}

unsafe impl raw_window_handle::HasRawDisplayHandle for MyWindowHandle {
    fn raw_display_handle(&self) -> raw_window_handle::RawDisplayHandle {
        raw_window_handle::RawDisplayHandle::from(raw_window_handle::WindowsDisplayHandle::empty())
    }
}

pub struct WebGPUState {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    nonmaterial_render_pipeline: wgpu::RenderPipeline,
    pulse_render_pipeline: wgpu::RenderPipeline,
    instances: Vec<InstanceRaw>,
    instance_buffer: wgpu::Buffer,
    nonmaterial_instance_buffer: wgpu::Buffer,
    pulse_instance_buffer: wgpu::Buffer,
    background_color: wgpu::Color,
    depth_texture: texture::Texture,
    camera_group: BindGroupData<CameraUniform>,
    light_group: BindGroupData<LightUniform>,
    start_time: Instant,
    time_group: BindGroupData<TimeUniform>,
    obj_model: model::Model,
}
impl WebGPUState {
    pub async fn new(window: HWND, hinstance: HINSTANCE, game_state: GameState) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let my_handle = MyWindowHandle::new(window, hinstance);
        let surface = unsafe { instance.create_surface(&my_handle) }.unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    label: None,
                    limits: wgpu::Limits::default(),
                },
                /* trace_path= */ None,
            )
            .await
            .unwrap();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);
        let mut rect: RECT = unsafe { mem::zeroed() };
        unsafe {
            let _ = GetClientRect(window, &mut rect);
        };
        let width = (rect.right - rect.left) as u32;
        let height = (rect.bottom - rect.top) as u32;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0], // TODO: how to get Pre/Postmultiplied in here?
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });
        let depth_texture = texture::create_depth_texture(&device, &config, "depth_texture");

        let camera_group = BindGroupData::<CameraUniform>::new(
            CameraUniform::from_camera(&game_state.get_camera()),
            &device,
            "Camera",
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        );

        let start_time = Instant::now();
        let time_group = BindGroupData::<TimeUniform>::new(
            TimeUniform::new(0.0),
            &device,
            "Time",
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        );

        let light_group = BindGroupData::<LightUniform>::new(
            LightUniform {
                position: [2.0, 2.0, 2.0],
                _padding: 0,
                color: [1.0, 1.0, 1.0],
                _padding2: 0,
            },
            &device,
            "Light",
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            wgpu::ShaderStages::VERTEX_FRAGMENT,
        );

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_group.layout,
                    &light_group.layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = {
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            };
            create_render_pipeline(
                "Render Pipeline",
                &device,
                &render_pipeline_layout,
                config.format,
                Some(texture::DEPTH_FORMAT),
                &[ModelVertex::describe_vb(), InstanceRaw::get_vertex_buffer_layout()],
                shader,
                "vs_main",
                "fs_main",
            )
        };
        let pulse_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Color Pulse Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_group.layout,
                    &light_group.layout,
                    &time_group.layout,
                ],
                push_constant_ranges: &[],
            });
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Color Pulse Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("color_pulse.wgsl").into()),
            };
            create_render_pipeline(
                "Color Pulse Render Pipeline",
                &device,
                &layout,
                config.format,
                Some(texture::DEPTH_FORMAT),
                &[ModelVertex::describe_vb(), InstanceRaw::get_vertex_buffer_layout()],
                shader,
                "vs_main",
                "fs_main",
            )
        };
        let nonmaterial_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Nonmaterial Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_group.layout,
                    &light_group.layout,
                ],
                push_constant_ranges: &[],
            });
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            };
            create_render_pipeline(
                "Nonmaterial Render Pipeline",
                &device,
                &layout,
                config.format,
                Some(texture::DEPTH_FORMAT),
                &[ModelVertex::describe_vb(), InstanceRaw::get_vertex_buffer_layout()],
                shader,
                "nonmaterial_vs_main",
                "nonmaterial_fs_main",
            )
        };

        let instances_raw =
            game_state.cube_instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instances_raw),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let mut nonmaterial_instances = Vec::<InstanceRaw>::new();
        nonmaterial_instances.push({
            let instance = Instance {
                position: cgmath::Vector3::<f32>::new(0.0, 0.0, 0.0),
                scale: 0.25,
                rotation: Rotor::identity(),
            };
            instance.to_raw()
        });
        let nonmaterial_instance_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&nonmaterial_instances),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let pulse_instances =
            game_state.pulse_instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let pulse_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Color Pulse Instance Buffer"),
            contents: bytemuck::cast_slice(&pulse_instances),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let obj_model = model::load_model("cube.obj", &device, &queue, &texture_bind_group_layout)
            .await
            .unwrap();

        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            nonmaterial_render_pipeline,
            pulse_render_pipeline,
            instances: instances_raw,
            instance_buffer,
            nonmaterial_instance_buffer,
            pulse_instance_buffer,
            background_color: wgpu::Color { r: 0.2, g: 0.5, b: 0.3, a: 1.0 },
            depth_texture,
            camera_group,
            light_group,
            start_time,
            time_group,
            obj_model,
        }
    }
    pub fn resize(&mut self, rect: RECT) {
        let w = (rect.right - rect.left) as u32;
        let h = (rect.bottom - rect.top) as u32;
        if w > 0 && h > 0 {
            self.config.width = (rect.right - rect.left) as u32;
            self.config.height = (rect.bottom - rect.top) as u32;
            self.surface.configure(&self.device, &self.config);
        }
        self.depth_texture =
            texture::create_depth_texture(&self.device, &self.config, "depth_texture");
    }
    pub fn update_bg_color(&mut self, point: &POINT) {
        self.background_color = wgpu::Color {
            r: (point.x as f64) / 2560.0,
            g: (point.y as f64) / 1440.0,
            b: 0.5 + 0.25 * (point.x * point.y) as f64 / (2560.0 * 1440.0),
            a: 1.0,
        };
        // Not necessary anymore: new model is we repeatedly call render in a loop.
        // let _ = self.render();
    }
    pub fn update_camera(&mut self, camera: Camera) {
        self.camera_group.uniform.update_view_proj(&camera);
        self.queue.write_buffer(
            &self.camera_group.buffer,
            0,
            bytemuck::cast_slice(&[self.camera_group.uniform]),
        );
        // Not necessary anymore: new model is we repeatedly call render in a loop.
        // let _ = self.render();
    }
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.background_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);

            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            render_pass.set_bind_group(1, &self.camera_group.bind_group, &[]);
            render_pass.set_bind_group(2, &self.light_group.bind_group, &[]);

            let mesh = &self.obj_model.meshes[0];
            let material = &self.obj_model.materials[mesh.material];
            draw_mesh_instanced(&mut render_pass, mesh, material, 0..self.instances.len() as u32);

            render_pass.set_pipeline(&self.nonmaterial_render_pipeline);

            render_pass.set_vertex_buffer(1, self.nonmaterial_instance_buffer.slice(..));

            render_pass.set_bind_group(1, &self.camera_group.bind_group, &[]);
            render_pass.set_bind_group(2, &self.light_group.bind_group, &[]);
            draw_nonmaterial_mesh_instanced(&mut render_pass, mesh, 0..1);

            render_pass.set_pipeline(&self.pulse_render_pipeline);
            render_pass.set_vertex_buffer(1, self.pulse_instance_buffer.slice(..));
            render_pass.set_bind_group(1, &self.camera_group.bind_group, &[]);
            render_pass.set_bind_group(2, &self.light_group.bind_group, &[]);
            render_pass.set_bind_group(3, &self.time_group.bind_group, &[]);
            let time = (Instant::now() - self.start_time).as_secs_f32();
            self.queue.write_buffer(&self.time_group.buffer, 0, bytemuck::cast_slice(&[time]));
            draw_mesh_instanced(&mut render_pass, mesh, material, 0..1);
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // BAD CODE ALERT: update the light's position each frame. I need to move this into the game
        // state. I'm just lazy right now.
        let old_position: cgmath::Vector3<_> = self.light_group.uniform.position.into();
        self.light_group.uniform.position =
            (cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(1.0))
                * old_position)
                .into();
        self.queue.write_buffer(
            &self.light_group.buffer,
            0,
            bytemuck::cast_slice(&[self.light_group.uniform]),
        );

        Ok(())
    }
}

fn create_render_pipeline(
    label: &str,
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: wgpu::ShaderModuleDescriptor,
    vertex_entrypoint: &str,
    fragment_entrypoint: &str,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(shader);
    let vertex = wgpu::VertexState {
        module: &shader,
        entry_point: vertex_entrypoint,
        buffers: vertex_layouts,
    };
    let color_target = [Some(wgpu::ColorTargetState {
        format: color_format,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    })];
    let fragment = Some(wgpu::FragmentState {
        module: &shader,
        entry_point: fragment_entrypoint,
        targets: &color_target,
    });
    let primitive = wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: Some(wgpu::Face::Back),
        // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
        polygon_mode: wgpu::PolygonMode::Fill,
        // Requires Features::DEPTH_CLIP_CONTROL
        unclipped_depth: false,
        // Requires Features::CONSERVATIVE_RASTERIZATION
        conservative: false,
    };
    let depth_stencil = depth_format.map(|format| wgpu::DepthStencilState {
        format,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::Less,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    });
    let multisample =
        wgpu::MultisampleState { count: 1, mask: !0, alpha_to_coverage_enabled: false };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex,
        fragment,
        primitive,
        depth_stencil,
        multisample,
        multiview: None,
    })
}

#[allow(unused)]
fn draw_mesh<'a>(render_pass: &mut wgpu::RenderPass<'a>, mesh: &'a Mesh, material: &'a Material) {
    draw_mesh_instanced(render_pass, mesh, material, 0..1);
}
fn draw_mesh_instanced<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    mesh: &'a Mesh,
    material: &'a Material,
    instances: Range<u32>,
) {
    render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
    render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    render_pass.set_bind_group(0, &material.bind_group, &[]);
    render_pass.draw_indexed(0..mesh.num_elements, 0, instances);
}
fn draw_nonmaterial_mesh_instanced<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    mesh: &'a Mesh,
    instances: Range<u32>,
) {
    render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
    render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    render_pass.draw_indexed(0..mesh.num_elements, 0, instances);
}

// Data for the graphics pipeline.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub model: [f32; 8],
}
impl InstanceRaw {
    fn get_vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Instance>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials,
                    // we'll be using 2, 3, and 4, for Vertex. We'll start at
                    // slot 5, not conflict with them later.
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

struct BindGroupData<T> {
    pub uniform: T,
    pub buffer: wgpu::Buffer,
    pub layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}
impl<T: bytemuck::Pod> BindGroupData<T> {
    pub fn new(
        uniform: T,
        device: &wgpu::Device,
        label: &str,
        usage: wgpu::BufferUsages,
        visibility: wgpu::ShaderStages,
    ) -> BindGroupData<T> {
        let uniform = uniform;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Buffer", label)),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: usage,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some(&format!("{} Bind Group Layout", label)),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("{} Bind Group", label)),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
        });
        BindGroupData { uniform, buffer, layout, bind_group }
    }
}
