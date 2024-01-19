// This hides the console window when launching cube.exe,
// at the cost of suppressing println! statements.
/* #![windows_subsystem = "windows"] */

mod texture;

use pollster::block_on;
use std::{
    ffi::c_void,
    mem::{self},
    result::Result,
};
use wgpu::util::DeviceExt;
use windows::Win32::{Foundation::POINT, System::LibraryLoader::GetModuleHandleA};
use windows::{
    core::*,
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::ValidateRect,
        UI::WindowsAndMessaging::*,
    },
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

/* State needed to interact with WebGPU / the GPU itself. */
#[allow(dead_code)]
struct WebGPUState {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    window_handle: MyWindowHandle,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    background_color: wgpu::Color,
    diffuse_bind_group: wgpu::BindGroup,
    diffuse_texture: texture::Texture,
}

impl WebGPUState {
    async fn new(window: HWND, hinstance: HINSTANCE) -> Self {
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
                /*trace_path=*/ None,
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
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let diffuse_bytes = include_bytes!("../assets/happy-tree.png");
        let diffuse_texture =
            texture::Texture::from_bytes(&device, &queue, diffuse_bytes, "happy-tree.png").unwrap();
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
        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::get_vertex_buffer_layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
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
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            window_handle: my_handle,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            background_color: wgpu::Color {
                r: 0.2,
                g: 0.5,
                b: 0.3,
                a: 1.0,
            },
            diffuse_bind_group,
            diffuse_texture,
        }
    }

    fn resize(&mut self, rect: RECT) {
        let w = (rect.right - rect.left) as u32;
        let h = (rect.bottom - rect.top) as u32;
        if w > 0 && h > 0 {
            self.config.width = (rect.right - rect.left) as u32;
            self.config.height = (rect.bottom - rect.top) as u32;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn update_bg_color(&mut self, point: &POINT) {
        self.background_color = wgpu::Color {
            r: (point.x as f64) / 2560.0,
            g: (point.y as f64) / 1440.0,
            b: 0.5 + 0.25 * (point.x * point.y) as f64 / (2560.0 * 1440.0),
            a: 1.0,
        };
        let _ = self.render();
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn main() -> windows::core::Result<()> {
    println!("Hello world!");

    unsafe {
        let hinstance = GetModuleHandleA(None)?;
        let window_class_name = s!("window");
        let wc = WNDCLASSA {
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hInstance: hinstance.into(),
            lpszClassName: window_class_name,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            cbWndExtra: std::mem::size_of::<*mut WebGPUState>() as i32,
            ..Default::default()
        };

        let atom = RegisterClassA(&wc);
        debug_assert!(atom != 0);

        let window = CreateWindowExA(
            WINDOW_EX_STYLE::default(),
            window_class_name,
            s!("My sample window"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            500,
            500,
            None,
            None,
            hinstance,
            None,
        );

        let state: WebGPUState = block_on(WebGPUState::new(window, hinstance.into()));
        SetWindowLongPtrA(
            window,
            WINDOW_LONG_PTR_INDEX(0),
            &state as *const WebGPUState as isize,
        );
        println!("initial state = {}", &state as *const WebGPUState as isize);

        let mut message = MSG::default();
        while GetMessageA(&mut message, None, 0, 0).into() {
            DispatchMessageA(&message);
        }

        Ok(())
    }
}

extern "system" fn wndproc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        let getptr = GetWindowLongPtrA(window, WINDOW_LONG_PTR_INDEX(0));
        let state: *mut WebGPUState = getptr as *mut WebGPUState;
        match message {
            WM_PAINT => {
                println!("WM_PAINT");
                println!("state = {}", getptr);
                let _ = (*state).render();
                ValidateRect(window, None);
                // let mut paint = MaybeUninit::<PAINTSTRUCT>::uninit();
                // let device_context = BeginPaint(window, paint.as_mut_ptr());
                // let paint = paint.assume_init();

                // static mut OP : ROP_CODE = WHITENESS;
                // PatBlt(device_context,
                //   paint.rcPaint.left,
                //   paint.rcPaint.top,
                //   paint.rcPaint.right - paint.rcPaint.left,
                //   paint.rcPaint.bottom - paint.rcPaint.top,
                //   OP);
                // if OP == WHITENESS {
                //   OP = BLACKNESS;
                // } else {
                //   OP = WHITENESS;
                // }
                // EndPaint(window, &paint);
                LRESULT(0)
            }
            WM_DESTROY => {
                println!("WM_DESTROY");
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_SIZE => {
                println!("WM_SIZE");
                if !state.is_null() {
                    let mut rect: RECT = mem::zeroed();
                    let _ = GetClientRect(window, &mut rect);
                    (*state).resize(rect);
                }
                LRESULT(0)
            }
            WM_MOUSEACTIVATE => {
                println!("WM_MOUSEACTIVATE");
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                println!("WM_MOUSEMOVE");
                if !state.is_null() {
                    let mut pt: POINT = mem::zeroed();
                    let _ = GetCursorPos(&mut pt);
                    (*state).update_bg_color(&pt);
                }
                LRESULT(0)
            }
            _ => DefWindowProcA(window, message, wparam, lparam),
        }
    }
}

// Data for the graphics pipeline

// Using the C repr is important here as we're using this in the vertex buffer, which crosses into the GPU boundary.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];
    fn get_vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.0868241, 0.49240386, 0.0],
        tex_coords: [0.4131759, 0.00759614],
    }, // A
    Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        tex_coords: [0.0048659444, 0.43041354],
    }, // B
    Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        tex_coords: [0.28081453, 0.949397],
    }, // C
    Vertex {
        position: [0.35966998, -0.3473291, 0.0],
        tex_coords: [0.85967, 0.84732914],
    }, // D
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        tex_coords: [0.9414737, 0.2652641],
    }, // E
];

const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];
