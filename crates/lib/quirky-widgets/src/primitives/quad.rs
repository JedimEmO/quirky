use crate::primitives::vertex::{Vertex, QUAD_INDEXES, QUAD_VERTICES};
use futures_signals::signal::ReadOnlyMutable;
use glam::UVec2;
use quirky::drawable_primitive::DrawablePrimitive;
use quirky::render_contexts::{PrepareContext, RenderContext};
use std::mem;
use std::sync::Arc;
use uuid::Uuid;
use wgpu::util::DeviceExt;
use wgpu::{include_wgsl, Device, PipelineLayoutDescriptor, RenderPipeline, VertexState};
use wgpu_macros::VertexLayout;

const QUAD_PIPELINE_ID: Uuid = Uuid::from_u128(0x94e8f8f8_e646_4e43_beef_c80b3688c998);

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
    geometry: ReadOnlyMutable<Arc<[Quad]>>,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
}

impl Quads {
    pub fn new(geometry: ReadOnlyMutable<Arc<[Quad]>>, device: &Device) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad index buffer"),
            contents: bytemuck::cast_slice(&QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&QUAD_INDEXES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad index buffer"),
            contents: bytemuck::cast_slice(geometry.lock_ref().as_ref()),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            geometry,
            instance_buffer,
            index_buffer,
            vertex_buffer,
        }
    }
}

impl DrawablePrimitive for Quads {
    fn prepare(&mut self, prepare_context: &mut PrepareContext) {
        if !prepare_context
            .pipeline_cache
            .contains_key(&QUAD_PIPELINE_ID)
        {
            prepare_context
                .pipeline_cache
                .insert(QUAD_PIPELINE_ID, configure_pipeline(prepare_context));
        }

        prepare_context.queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(self.geometry.lock_ref().as_ref()),
        )
    }

    fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, render_context: &RenderContext<'a>) {
        let pipeline = render_context
            .pipeline_cache
            .get(&QUAD_PIPELINE_ID)
            .unwrap();

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, render_context.camera_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..6, 0, 0..(self.geometry.lock_ref().len() as u32));
    }
}

fn configure_pipeline(prepare_context: &PrepareContext) -> RenderPipeline {
    let pipeline_layout =
        prepare_context
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[prepare_context.camera_bind_group_layout],
                push_constant_ranges: &[],
            });

    let shader = prepare_context
        .device
        .create_shader_module(include_wgsl!("shaders/quad.wgsl"));

    prepare_context
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    format: prepare_context.surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        })
}
