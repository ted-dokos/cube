use crate::resources;

use anyhow::Result;
use std::io::Cursor;
use std::{io::BufReader, mem::size_of};
use wgpu::{
    BindGroupLayout, Buffer, BufferAddress, Device, Queue, VertexAttribute, VertexBufferLayout,
    VertexFormat, VertexStepMode,
};
use wgpu::util::DeviceExt;

use crate::texture;

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub num_elements: u32,
    pub material: usize,
    pub raw_vertices: Vec<ModelVertex>,
    pub raw_indices: Vec<u32>,
}
pub trait DescribeVB {
    fn describe_vb() -> VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}
impl DescribeVB for ModelVertex {
    fn describe_vb() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: size_of::<ModelVertex>() as BufferAddress,
            attributes: &[
                VertexAttribute { format: VertexFormat::Float32x3, offset: 0, shader_location: 0 },
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: size_of::<[f32; 5]>() as BufferAddress,
                    shader_location: 2,
                },
            ],
            step_mode: VertexStepMode::Vertex,
        }
    }
}

pub fn cube_model(
    device: &Device,
) -> Model {
    let vertices: Vec<ModelVertex> = [
        ModelVertex { position: [1.0, 1.0, 1.0], tex_coords: [1.0, 1.0], normal: [1.0, 0.0, 0.0]}, // 0
        ModelVertex { position: [1.0, -1.0, 1.0], tex_coords: [1.0, -1.0], normal: [1.0, 0.0, 0.0]},
        ModelVertex { position: [1.0, 1.0, -1.0], tex_coords: [-1.0, 1.0], normal: [1.0, 0.0, 0.0]},
        ModelVertex { position: [1.0, -1.0, -1.0], tex_coords: [-1.0, -1.0], normal: [1.0, 0.0, 0.0]},

        ModelVertex { position: [1.0, 1.0, 1.0], tex_coords: [1.0, 1.0], normal: [0.0, 1.0, 0.0]}, // 4
        ModelVertex { position: [-1.0, 1.0, 1.0], tex_coords: [-1.0, 1.0], normal: [0.0, 1.0, 0.0]},
        ModelVertex { position: [1.0, 1.0, -1.0], tex_coords: [1.0, -1.0], normal: [0.0, 1.0, 0.0]},
        ModelVertex { position: [-1.0, 1.0, -1.0], tex_coords: [-1.0, -1.0], normal: [0.0, 1.0, 0.0]},

        ModelVertex { position: [1.0, 1.0, 1.0], tex_coords: [1.0, 1.0], normal: [0.0, 0.0, 1.0]}, // 8
        ModelVertex { position: [-1.0, 1.0, 1.0], tex_coords: [-1.0, 1.0], normal: [0.0, 0.0, 1.0]},
        ModelVertex { position: [1.0, -1.0, 1.0], tex_coords: [1.0, -1.0], normal: [0.0, 0.0, 1.0]},
        ModelVertex { position: [-1.0, -1.0, 1.0], tex_coords: [-1.0, -1.0], normal: [0.0, 0.0, 1.0]},

        ModelVertex { position: [-1.0, -1.0, -1.0], tex_coords: [-1.0, -1.0], normal: [-1.0, 0.0, 0.0]}, // 12
        ModelVertex { position: [-1.0, 1.0, -1.0], tex_coords: [-1.0, 1.0], normal: [-1.0, 0.0, 0.0]},
        ModelVertex { position: [-1.0, -1.0, 1.0], tex_coords: [1.0, -1.0], normal: [-1.0, 0.0, 0.0]},
        ModelVertex { position: [-1.0, 1.0, 1.0], tex_coords: [1.0, 1.0], normal: [-1.0, 0.0, 0.0]},

        ModelVertex { position: [-1.0, -1.0, -1.0], tex_coords: [-1.0, -1.0], normal: [0.0, -1.0, 0.0]}, //16
        ModelVertex { position: [1.0, -1.0, -1.0], tex_coords: [1.0, -1.0], normal: [0.0, -1.0, 0.0]},
        ModelVertex { position: [-1.0, -1.0, 1.0], tex_coords: [-1.0, 1.0], normal: [0.0, -1.0, 0.0]},
        ModelVertex { position: [1.0, -1.0, 1.0], tex_coords: [1.0, 1.0], normal: [0.0, -1.0, 0.0]},

        ModelVertex { position: [-1.0, -1.0, -1.0], tex_coords: [-1.0, -1.0], normal: [0.0, 0.0, -1.0]}, // 20
        ModelVertex { position: [1.0, -1.0, -1.0], tex_coords: [1.0, -1.0], normal: [0.0, 0.0, -1.0]},
        ModelVertex { position: [-1.0, 1.0, -1.0], tex_coords: [-1.0, 1.0], normal: [0.0, 0.0, -1.0]},
        ModelVertex { position: [1.0, 1.0, -1.0], tex_coords: [1.0, 1.0], normal: [0.0, 0.0, -1.0]},
    ].into();
    let indices: [u32; 36] = [
        0, 1, 2,
        1, 3, 2,

        4, 6, 5,
        5, 6, 7,

        8, 9, 10,
        10, 9, 11,

        12, 14, 13,
        13, 14, 15,

        16, 17, 18,
        18, 17, 19,

        20, 22, 21,
        21, 22, 23,
    ];
    let name = "Simple_Cube";
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{:?} Vertex Buffer", name)),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{:?} Index Buffer", name)),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    let mesh = Mesh {
        name: name.to_string(),
        vertex_buffer,
        index_buffer,
        num_elements: indices.len() as u32,
        material: 0,
        raw_vertices: vertices,
        raw_indices: indices.into(),
    };
    Model {
        materials: [].into(),
        meshes: [mesh].into(),
    }
}

pub async fn load_model(
    file_name: &str,
    device: &Device,
    queue: &Queue,
    layout: &BindGroupLayout,
) -> Result<Model> {
    let obj_text = resources::load_string(file_name).await?;
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);

    let (models, obj_materials) = tobj::load_obj_buf_async(
        &mut obj_reader,
        &tobj::LoadOptions { triangulate: true, single_index: true, ..Default::default() },
        |p| async move {
            let mat_text = resources::load_string(&p).await.unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        },
    )
    .await?;

    let mut materials: Vec<Material> = Vec::new();
    for m in obj_materials? {
        let diffuse_texture =
            resources::load_texture(&(m.diffuse_texture.unwrap()), device, queue).await?;
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
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
            label: None,
        });

        materials.push(Material { name: m.name, diffuse_texture, bind_group })
    }

    let meshes = models
        .into_iter()
        .map(|m| {
            let vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ],
                })
                .collect::<Vec<_>>();

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", file_name)),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", file_name)),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            Mesh {
                name: file_name.to_string(),
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material: m.mesh.material_id.unwrap_or(0),
                raw_vertices: vertices,
                raw_indices: m.mesh.indices.clone(),
            }
        })
        .collect::<Vec<_>>();

    Ok(Model { meshes, materials })
}
