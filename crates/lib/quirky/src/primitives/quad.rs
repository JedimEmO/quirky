use std::mem;
use std::sync::Arc;
use glam::UVec2;
use once_cell::sync::OnceCell;
use wgpu::{BindGroupLayout, Device, include_wgsl, PipelineLayoutDescriptor, RenderPipeline, TextureFormat, VertexState};
use wgpu::util::DeviceExt;
use wgpu_macros::VertexLayout;
use crate::primitives::{DrawablePrimitive, Primitive, RenderContext};

static QUAD_PIPELINE: OnceCell<Arc<RenderPipeline>> = OnceCell::new();

const INDEXES: [u16; 6] = [0, 1, 2, 0, 2, 3];

#[derive(VertexLayout, bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
#[repr(C)]
pub struct Vertex {
    pos: [f32; 2],
}

const VERTICES: [Vertex; 4] = [
    Vertex { pos: [0.0, 0.0] },
    Vertex { pos: [1.0, 0.0] },
    Vertex { pos: [1.0, 1.0] },
    Vertex { pos: [0.0, 1.0] },
];

#[repr(C)]
#[derive(VertexLayout, bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
#[layout(Instance)]
pub struct Quad {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
}

impl Quad {
    pub fn new(pos: UVec2, size: UVec2, color: [f32; 4]) -> Self {
        Self {
            color,
            pos: pos.as_vec2().to_array(),
            size: size.as_vec2().to_array(),
        }
    }

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Quad>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct Quads {
    num_instances: u32,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
}

impl Quads {
    pub fn new(geometry: Vec<Quad>, device: &Device) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad index buffer"),
            contents: bytemuck::cast_slice(&VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&INDEXES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad index buffer"),
            contents: bytemuck::cast_slice(&geometry),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            num_instances: geometry.len() as u32,
            instance_buffer,
            index_buffer,
            vertex_buffer,
        }
    }
}

impl DrawablePrimitive for Quads {
    fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, render_context: &RenderContext<'a>) {
        if let Some(pipeline) = QUAD_PIPELINE.get() {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &render_context.camera_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..6, 0, 0..self.num_instances);
        } else {
            println!("QUAD_PIPELINE missing!");
        }
    }
}

impl Primitive for Quads {
    fn configure_pipeline(
        device: &Device,
        bind_group_layouts: &[&BindGroupLayout],
        surface_format: TextureFormat,
    ) {
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts,
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(include_wgsl!("shaders/quad.wgsl"));

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::LAYOUT, Quad::layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        let pipeline = Arc::new(pipeline);

        QUAD_PIPELINE
            .set(pipeline)
            .expect("failed to set QUAD_PIPELINE");
    }
}
