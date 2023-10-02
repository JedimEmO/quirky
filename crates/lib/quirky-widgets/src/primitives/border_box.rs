use crate::primitives::vertex::{Vertex, QUAD_INDEXES, QUAD_VERTICES};
use futures_signals::signal::ReadOnlyMutable;
use quirky::drawable_primitive::DrawablePrimitive;
use quirky::render_contexts::{PrepareContext, RenderContext};
use std::mem;
use uuid::Uuid;
use wgpu::util::DeviceExt;
use wgpu::{
    include_wgsl, Device, PipelineLayoutDescriptor, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, VertexState,
};
use wgpu_macros::VertexLayout;

const BORDER_BOX_PRIMITIVE_UUID: Uuid = Uuid::from_u128(0xe136d9b9_d64a_4932_8eb3_106b52e2537c);

#[repr(C)]
#[derive(VertexLayout, bytemuck::Pod, bytemuck::Zeroable, Copy, Clone, Default)]
#[layout(Instance)]
pub struct BorderBoxData {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub shade_color: [f32; 4],
    pub border_side: u32,
    pub borders: [u32; 4],
}

impl BorderBoxData {
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<BorderBoxData>() as wgpu::BufferAddress,
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
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<u32>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Uint32,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[u32; 4]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Uint32x4,
                },
            ],
        }
    }
}

pub struct BorderBox {
    data: ReadOnlyMutable<BorderBoxData>,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
}

impl BorderBox {
    pub fn new(data: ReadOnlyMutable<BorderBoxData>, device: &Device) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BorderBox data vertex buffer"),
            contents: bytemuck::cast_slice(&QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&QUAD_INDEXES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BorderBox data buffer"),
            contents: bytemuck::cast_slice(&[data.get()]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            data,
            index_buffer,
            instance_buffer,
            vertex_buffer,
        }
    }
}

impl DrawablePrimitive for BorderBox {
    fn prepare(&mut self, prepare_context: &mut PrepareContext) -> () {
        if !prepare_context
            .pipeline_cache
            .contains_key(&BORDER_BOX_PRIMITIVE_UUID)
        {
            prepare_context.pipeline_cache.insert(
                BORDER_BOX_PRIMITIVE_UUID,
                create_border_box_pipeline(prepare_context),
            );
        }

        prepare_context.queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&[self.data.get()]),
        )
    }

    fn draw<'a>(&'a self, pass: &mut RenderPass<'a>, render_context: &RenderContext<'a>) {
        let pipeline = render_context
            .pipeline_cache
            .get(&BORDER_BOX_PRIMITIVE_UUID)
            .unwrap();

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &render_context.camera_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..6, 0, 0..1);
    }
}

fn create_border_box_pipeline(render_context: &PrepareContext) -> RenderPipeline {
    let pipeline_layout = render_context
        .device
        .create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[render_context.camera_bind_group_layout],
            push_constant_ranges: &[],
        });

    let shader = render_context
        .device
        .create_shader_module(include_wgsl!("shaders/border_box.wgsl"));

    render_context
        .device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::LAYOUT, BorderBoxData::layout()],
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
                    format: render_context.surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        })
}
