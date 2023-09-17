use crate::primitives::{DrawablePrimitive, PrepareContext, RenderContext};
use crate::LayoutBox;
use image::RgbaImage;
use std::mem;
use uuid::Uuid;
use wgpu::util::DeviceExt;
use wgpu::{include_wgsl, Buffer, RenderPass, TextureDimension, TextureFormat, VertexState};
use wgpu_macros::VertexLayout;

static PRIMITIVE_UUID: Uuid = Uuid::from_u128(0x96f1543e_52e7_4f6b_8dc9_c5561df1f404);

const INDEXES: [u16; 6] = [0, 1, 2, 0, 2, 3];

#[derive(VertexLayout, bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
#[repr(C)]
struct Vertex {
    pos: [f32; 2],
    tex_coords: [f32; 2],
}

const VERTICES: [Vertex; 4] = [
    Vertex {
        pos: [0.0, 0.0],
        tex_coords: [0.0, 0.0],
    },
    Vertex {
        pos: [1.0, 0.0],
        tex_coords: [1.0, 0.0],
    },
    Vertex {
        pos: [1.0, 1.0],
        tex_coords: [1.0, 1.0],
    },
    Vertex {
        pos: [0.0, 1.0],
        tex_coords: [0.0, 1.0],
    },
];

#[repr(C)]
#[derive(VertexLayout, bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
#[layout(Instance)]
struct Quad {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
}

impl Quad {
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Quad>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct ImagePrimitive {
    pub data: RgbaImage,
    pub vertex_buffer: Option<Buffer>,
    pub index_buffer: Option<Buffer>,
    pub instance_buffer: Option<Buffer>,
    pub bb: LayoutBox,
}

impl DrawablePrimitive for ImagePrimitive {
    fn prepare(&mut self, render_context: &mut PrepareContext) -> () {
        let dimensions = self.data.dimensions();
        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = render_context
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("image primitive texture"),
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

        render_context.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        let diffuse_texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let diffuse_sampler = render_context
            .device
            .create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

        let texture_bind_group_layout =
            render_context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let diffuse_bind_group =
            render_context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                        },
                    ],
                    label: Some("diffuse_bind_group"),
                });

        let render_pipeline_layout =
            render_context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[
                        &render_context.camera_bind_group_layout,
                        &texture_bind_group_layout,
                    ], // NEW!
                    push_constant_ranges: &[],
                });

        let shader = render_context
            .device
            .create_shader_module(include_wgsl!("shaders/textured_quad.wgsl"));

        let pipeline =
            render_context
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: None,
                    layout: Some(&render_pipeline_layout),
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
                            format: render_context.surface_format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                });

        let vertex_buffer =
            render_context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("quad index buffer"),
                    contents: bytemuck::cast_slice(&VERTICES),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        let index_buffer =
            render_context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&INDEXES),
                    usage: wgpu::BufferUsages::INDEX,
                });

        let instance_buffer =
            render_context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("quad index buffer"),
                    contents: bytemuck::cast_slice(&[Quad {
                        pos: *self.bb.pos.as_vec2().as_ref(),
                        size: *self.bb.size.as_vec2().as_ref(),
                        color: [0.0, 0.0, 0.0, 0.0],
                    }]),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
        self.instance_buffer = Some(instance_buffer);

        render_context
            .pipeline_cache
            .insert(PRIMITIVE_UUID, pipeline);

        render_context
            .bind_group_cache
            .insert(PRIMITIVE_UUID, diffuse_bind_group);
    }

    fn draw<'a>(&'a self, pass: &mut RenderPass<'a>, ctx: &RenderContext<'a>) {
        let pipeline = ctx.pipeline_cache.get(&PRIMITIVE_UUID).unwrap();
        let bind_group = ctx.bind_group_cache.get(&PRIMITIVE_UUID).unwrap();

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, ctx.camera_bind_group, &[]);
        pass.set_bind_group(1, bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.as_ref().unwrap().slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.as_ref().unwrap().slice(..));
        pass.set_index_buffer(
            self.index_buffer.as_ref().unwrap().slice(..),
            wgpu::IndexFormat::Uint16,
        );
        pass.draw_indexed(0..6, 0, 0..1);
    }
}
