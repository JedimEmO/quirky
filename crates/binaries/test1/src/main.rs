use futures::StreamExt;
use futures_signals::signal::Mutable;
use futures_signals::signal::SignalExt;

use glam::{uvec2, vec3, UVec2, Vec4};
use quirky::LayoutBox;
use std::iter;
use std::sync::{Arc, Mutex};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferBindingType, BufferUsages, Color, CommandEncoder,
    Device, PipelineLayoutDescriptor, RenderPass, ShaderStages, Texture, TextureView, VertexState,
};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

#[tokio::main]
async fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let size = window.inner_size();
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        dx12_shader_compiler: Default::default(),
    });

    let surface = unsafe { instance.create_surface(&window) }.unwrap();

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: Default::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .unwrap();

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(surface_caps.formats[0]);

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
    };

    surface.configure(&device, &config);
    let _pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let shader = device.create_shader_module(include_wgsl!("quad.wgsl"));

    let mut ui_camera = UiCamera2D::default();
    ui_camera.resize_viewport(uvec2(800, 600));

    let camera_uniform = ui_camera.create_camera_uniform();

    let camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("camera buffer"),
        contents: bytemuck::cast_slice(&[camera_uniform]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let camera_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: None,
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &camera_bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&camera_bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[Vertex::buffer_layout()],
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
                format: config.format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    });

    let num_quads = Mutable::new(3);

    let boxes_to_draw = num_quads.signal().map(|count| {
        (0..count)
            .map(|c| LayoutBox {
                pos: uvec2(10, 10 + 30 * c),
                size: uvec2(100, 20),
            })
            .collect::<Vec<_>>()
    });

    let mut quads: Vec<Quad> = vec![];
    let requested_boxes = Arc::new(Mutex::<Option<Vec<LayoutBox>>>::new(None));
    let elproxy = event_loop.create_proxy();

    tokio::spawn({
        let requested_boxes = requested_boxes.clone();
        async move {
            let mut strm = boxes_to_draw.to_stream();
            while let Some(b) = strm.next().await {
                let requested_boxes = requested_boxes.clone();

                let mut lock = requested_boxes.lock().unwrap();

                let _ = lock.insert(b);
                elproxy.send_event(()).unwrap();
            }
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(new_size) => {
                    if new_size.height > 0 && new_size.width > 0 {
                        config.width = new_size.width;
                        config.height = new_size.height;
                        let next_count = config.height / 100;
                        num_quads.set(next_count);
                        surface.configure(&device, &config);
                        ui_camera.resize_viewport(UVec2::new(config.width, config.height));
                        let camera_uniform = ui_camera.create_camera_uniform();
                        queue.write_buffer(
                            &camera_buffer,
                            0,
                            bytemuck::cast_slice(&[camera_uniform]),
                        )
                    }
                }
                _ => {}
            },
            Event::RedrawRequested(_d) => {
                let output = surface.get_current_texture().unwrap();
                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("hi there"),
                });

                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 1.0,
                                    g: 0.5,
                                    b: 0.5,
                                    a: 1.0,
                                }),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                    pass.set_pipeline(&pipeline);
                    pass.set_bind_group(0, &camera_bind_group, &[]);

                    if let Some(incoming_boxes) = requested_boxes.lock().unwrap().take() {
                        quads.clear();

                        quads.append(
                            &mut incoming_boxes
                                .into_iter()
                                .map(|b| Quad::new(&device, b.pos, b.size))
                                .collect::<Vec<_>>(),
                        )
                    }

                    for quad in quads.iter() {
                        quad.draw(&mut pass);
                    }
                }

                queue.submit(iter::once(encoder.finish()));

                output.present();
            }
            _ => {}
        }
    });
}

pub struct UiAppSettings {
    pub background_color: Color,
}

pub fn ui_render_pass(target_texture: &Texture, device: &Device, ui_app_settings: &UiAppSettings) {
    let view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("hi there"),
    });

    let _render_pass = clear_render_pass(&mut encoder, &view, ui_app_settings.background_color);
}

pub fn clear_render_pass<'a>(
    encoder: &'a mut CommandEncoder,
    view: &'a TextureView,
    color: Color,
) -> RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(color),
                store: true,
            },
        })],
        depth_stencil_attachment: None,
    })
}

fn draw_quads(_quads: &[Quad], _render_pass: &mut RenderPass, _device: &Device) {}

#[derive(bytemuck::Zeroable, bytemuck::Pod, Copy, Clone)]
#[repr(C)]
struct Vertex {
    pos: [f32; 2],
}

impl Vertex {
    fn buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
            }],
        }
    }
}

struct Quad {
    pub vertices: wgpu::Buffer,
    pub indexes: wgpu::Buffer,
}

const INDEXES: [u16; 6] = [0, 1, 2, 0, 2, 3];

impl Quad {
    pub fn new(device: &wgpu::Device, pos: UVec2, size: UVec2) -> Self {
        let left_x = pos.x as f32;
        let right_x = (pos.x + size.x) as f32;
        let top_y = pos.y as f32;
        let bottom_y = (pos.y + size.y) as f32;

        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad buffer"),
            contents: bytemuck::cast_slice(&[
                Vertex {
                    pos: [left_x, top_y],
                },
                Vertex {
                    pos: [right_x, top_y],
                },
                Vertex {
                    pos: [right_x, bottom_y],
                },
                Vertex {
                    pos: [left_x, bottom_y],
                },
            ]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let indexes = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad index buffer"),
            contents: bytemuck::cast_slice(&INDEXES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self { vertices, indexes }
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        // pass.set_scissor_rect(20, 20, 100, 20);
        pass.set_index_buffer(self.indexes.slice(..), wgpu::IndexFormat::Uint16);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw_indexed(0..6, 0, 0..1);
    }
}

#[derive(bytemuck::Zeroable, bytemuck::Pod, Copy, Clone)]
#[repr(C)]
pub struct UiCameraUniform {
    pub transform: [[f32; 4]; 4],
}

#[derive(Default)]
struct UiCamera2D {
    transform: glam::Mat4,
}

impl UiCamera2D {
    pub fn create_camera_uniform(&self) -> UiCameraUniform {
        UiCameraUniform {
            transform: self.transform.to_cols_array_2d(),
        }
    }

    pub fn resize_viewport(&mut self, size: UVec2) {
        let camera_origin_translate = glam::Mat4::from_translation(vec3(-1.0, 1.0, 0.0));

        let camera_coordinate_transform = glam::mat4(
            Vec4::new(1.0, 0.0, 0.0, 0.0),
            Vec4::new(0.0, -1.0, 0.0, 0.0),
            Vec4::new(0.0, -1.0, 0.0, 0.0),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        );

        let ndc_scale = glam::mat4(
            Vec4::new(2.0 / size.x as f32, 0.0, 0.0, 0.0),
            Vec4::new(0.0, 2.0 / size.y as f32, 0.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 0.0),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        );

        self.transform = camera_origin_translate * ndc_scale * camera_coordinate_transform;
    }

    pub fn transform(&self) -> &glam::Mat4 {
        &self.transform
    }
}

#[cfg(test)]
mod test {
    use quirky::assert_f32_eq;

    use crate::UiCamera2D;
    use glam::{vec4, UVec2, Vec4};

    #[test]
    fn test_camera_transform() {
        let mut cam = UiCamera2D::default();

        cam.resize_viewport(UVec2::new(800, 600));

        let points_to_test = vec![
            (vec4(0.0, 0.0, 0.0, 1.0), vec4(-1.0, 1.0, 0.0, 0.0)),
            (vec4(800.0, 600.0, 0.0, 1.0), vec4(1.0, -1.0, 0.0, 0.0)),
            (vec4(0.0, 600.0, 0.0, 1.0), vec4(-1.0, -1.0, 0.0, 0.0)),
            (vec4(800.0, 0.0, 0.0, 1.0), vec4(1.0, 1.0, 0.0, 0.0)),
        ];

        points_to_test.iter().for_each(|p| {
            check_transform(p.0, p.1, &cam);
        });
    }

    fn check_transform(pixel_coord: Vec4, ndc_space: Vec4, cam: &UiCamera2D) {
        let pixel_ndc = cam.transform().mul_vec4(pixel_coord);

        assert_f32_eq!(pixel_ndc.x, ndc_space.x, "ndc x");
        assert_f32_eq!(pixel_ndc.y, ndc_space.y, "ndc y");

        let reverse_transform = cam.transform().inverse();
        let pixel_coord_rev = reverse_transform.mul_vec4(pixel_ndc);

        assert_f32_eq!(pixel_coord_rev.x, pixel_coord.x, "rev px x");
        assert_f32_eq!(pixel_coord_rev.y, pixel_coord.y, "rev px y");
    }
}

