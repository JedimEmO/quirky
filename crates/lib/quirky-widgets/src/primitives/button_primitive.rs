use crate::widgets::button::ButtonState;
use futures_signals::signal::ReadOnlyMutable;
use glam::UVec2;
use quirky::primitives::quad::QUAD_INDEXES;
use quirky::primitives::vertex::{Vertex, VERTICES};
use quirky::primitives::{DrawablePrimitive, PrepareContext, RenderContext};
use std::mem;
use uuid::Uuid;
use wgpu::util::DeviceExt;
use wgpu::{
    include_wgsl, Device, PipelineLayoutDescriptor, RenderPipeline, RenderPipelineDescriptor,
    VertexState,
};
use wgpu_macros::VertexLayout;

static BUTTON_PRIMITIVE_UUID: Uuid = Uuid::from_u128(0x2310e3e8_eda0_4321_96d5_44b4f42c24b0);

#[repr(C)]
#[derive(VertexLayout, bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
#[layout(Instance)]
pub struct ButtonData {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
}

impl Default for ButtonData {
    fn default() -> Self {
        Self {
            pos: [0.0, 0.0],
            size: [0.0, 0.0],
            color: [0.01, 0.02, 0.03, 1.0],
        }
    }
}

impl ButtonData {
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ButtonData>() as wgpu::BufferAddress,
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

pub struct ButtonPrimitive {
    button_data: ReadOnlyMutable<ButtonData>,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
}

impl ButtonPrimitive {
    pub fn new(button_data: ReadOnlyMutable<ButtonData>, device: &Device) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("button data vertex buffer"),
            contents: bytemuck::cast_slice(&VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&QUAD_INDEXES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("button data buffer"),
            contents: bytemuck::cast_slice(&[button_data.get()]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            button_data,
            index_buffer,
            instance_buffer,
            vertex_buffer,
        }
    }
}

impl DrawablePrimitive for ButtonPrimitive {
    fn prepare(&mut self, render_context: &mut PrepareContext) -> () {
        if !render_context
            .pipeline_cache
            .contains_key(&BUTTON_PRIMITIVE_UUID)
        {
            render_context.pipeline_cache.insert(
                BUTTON_PRIMITIVE_UUID,
                create_button_pipeline(render_context),
            );
        }

        render_context.queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&[self.button_data.get()]),
        )
    }

    fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, render_context: &RenderContext<'a>) {
        let pipeline = render_context
            .pipeline_cache
            .get(&BUTTON_PRIMITIVE_UUID)
            .unwrap();

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &render_context.camera_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..6, 0, 0..1);
    }
}

fn create_button_pipeline(render_context: &PrepareContext) -> RenderPipeline {
    let pipeline_layout = render_context
        .device
        .create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[render_context.camera_bind_group_layout],
            push_constant_ranges: &[],
        });

    let shader = render_context
        .device
        .create_shader_module(include_wgsl!("shaders/button.wgsl"));

    render_context
        .device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::LAYOUT, ButtonData::layout()],
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
