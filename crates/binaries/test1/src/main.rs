use futures::stream::FuturesUnordered;
use futures::{select, FutureExt, SinkExt, Stream, StreamExt};
use futures_signals::signal::Mutable;
use futures_signals::signal::{always, SignalExt};
use std::future::Future;

use futures_signals::signal_vec::{MutableVec, SignalVecExt};
use glam::{uvec2, vec3, UVec2, Vec4};
use quirky::drawables::Drawable;
use quirky::widget::widgets::{List, Slab};
use quirky::{clone, run_widgets, LayoutBox, SizeConstraint};
use std::iter;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferBindingType, BufferUsages, Color, CommandEncoder,
    Device, PipelineLayoutDescriptor, RenderPass, ShaderStages, Texture, TextureView, VertexState,
};

use quirky::primitives::{Quad, Quads, Vertex};
use quirky::widget::Widget;
use quirky::widgets::box_layout::{BoxLayout, ChildDirection};
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

#[async_recursion::async_recursion]
async fn drawable_tree_watch_inner(
    drawables: MutableVec<Drawable>,
    mut tx: futures::channel::mpsc::Sender<()>,
) {
    let mut drawables_stream = drawables
        .signal_vec_cloned()
        .to_signal_cloned()
        .to_stream()
        .fuse();
    let mut futures = FuturesUnordered::new();

    loop {
        let mut next_drawables = drawables_stream.select_next_some();
        let mut next_unordered = futures.select_next_some();

        select! {
                drawables = next_drawables => {
                    tx.send(()).await.expect("failed to send drawables notification");
                    futures = FuturesUnordered::new();

                   for drawable in drawables {
                        match drawable {
                            Drawable::SubTree{children, ..} => {
                                futures.push(drawable_tree_watch_inner(children.clone(), tx.clone()));
                            }
                            _ => {}
                        }
                    }
                }
                _ = next_unordered => {}
        }
    }
}

pub fn drawable_tree_watch(
    widgets: MutableVec<Drawable>,
) -> (Pin<Box<impl Stream<Item = ()>>>, impl Future<Output = ()>) {
    let (tx, rx) = futures::channel::mpsc::channel(100);

    let fut = drawable_tree_watch_inner(widgets, tx);

    let rx = Box::pin(
        futures_signals::signal::from_stream(rx)
            .throttle(|| sleep(Duration::from_millis(50)))
            .map(|_| ())
            .to_stream(),
    );
    (rx, fut)
}

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

    let device = Box::leak(Box::new(device));

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

    surface.configure(device, &config);

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
                format: config.format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    });

    let num_quads = Mutable::new(3);

    let mut quads: Vec<Arc<Quads>> = vec![];
    let requested_drawables = Arc::new(Mutex::<Option<Vec<Drawable>>>::new(None));
    let elproxy = event_loop.create_proxy();

    let children: Mutable<Vec<Arc<dyn Widget>>> = Mutable::new(vec![
        Arc::new(List {
            children: Mutable::new(
                (0..6)
                    .map(|_| Arc::new(Slab::default()) as Arc<dyn Widget>)
                    .collect(),
            ),
            requested_size: Mutable::new(SizeConstraint::MinSize(UVec2::new(300, 200))),
            bounding_box: Default::default(),
            background: Some([1.0, 0.4, 0.4, 1.0]),
        }),
        Arc::new(List {
            children: Mutable::new(
                (0..6)
                    .map(|_| Arc::new(Slab::default()) as Arc<dyn Widget>)
                    .collect(),
            ),
            requested_size: Mutable::new(SizeConstraint::Unconstrained),
            bounding_box: Default::default(),
            background: Some([0.4, 1.0, 0.4, 1.0]),
        }),
    ]);

    let bb = Mutable::new(LayoutBox {
        pos: UVec2::new(0, 0),
        size: UVec2::new(600, 400),
    });

    let boxed_layout = Arc::new(
        BoxLayout::builder()
            .children(clone!(children, move || children.signal_cloned()))
            .child_direction(|| always(ChildDirection::Horizontal))
            .size_constraint(|| always(SizeConstraint::Unconstrained))
            .bounding_box(bb.clone())
            .build(),
    );

    {
        let widgets: MutableVec<Arc<dyn Widget>> = MutableVec::new_with_values(vec![boxed_layout]);
        let (drawables, fut) = run_widgets(widgets.clone(), device);
        let (out, drawables_watch_fut) = drawable_tree_watch(drawables.clone());

        tokio::spawn(drawables_watch_fut);
        tokio::spawn(fut);

        tokio::spawn(clone!(
            requested_drawables,
            clone!(drawables, async move {
                let mut out = out.fuse();

                loop {
                    let _v = out.next().await;

                    let _ = requested_drawables
                        .lock()
                        .unwrap()
                        .insert(drawables.lock_ref().to_vec());

                    elproxy
                        .send_event(())
                        .expect("failed to send eventloop message on new drawables");
                }
            })
        ));

        let device = &*device;
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::UserEvent(()) => {
                    window.request_redraw();
                }
                Event::WindowEvent { event, window_id } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(new_size) => {
                            if new_size.height > 0 && new_size.width > 0 {
                                config.width = new_size.width;
                                config.height = new_size.height;
                                bb.set(LayoutBox {
                                    pos: Default::default(),
                                    size: UVec2::new(config.width, config.height),
                                });
                                let next_count = config.height / 100;
                                num_quads.set(next_count);
                                surface.configure(device, &config);
                                ui_camera.resize_viewport(UVec2::new(config.width, config.height));
                                let camera_uniform = ui_camera.create_camera_uniform();
                                queue.write_buffer(
                                    &camera_buffer,
                                    0,
                                    bytemuck::cast_slice(&[camera_uniform]),
                                )
                            }
                        }
                        WindowEvent::KeyboardInput { input, .. } => {
                            if input.state == ElementState::Pressed {
                                match input.scancode {
                                    30 => children.lock_mut().push(Arc::new(Slab::default())), // s
                                    48 => children.lock_mut().push(Arc::new(List {
                                        children: Mutable::new(
                                            (0..6)
                                                .map(|_| {
                                                    Arc::new(Slab::default()) as Arc<dyn Widget>
                                                })
                                                .collect(),
                                        ),
                                        requested_size: Default::default(),
                                        bounding_box: Default::default(),
                                        background: Some([0.4, 0.4, 0.4, 1.0]),
                                    })
                                        as Arc<dyn Widget>), // b
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }

                Event::RedrawRequested(_d) => {
                    let output = surface.get_current_texture().unwrap();
                    let view = output
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
                                        r: 0.05,
                                        g: 0.05,
                                        b: 0.05,
                                        a: 0.5,
                                    }),
                                    store: true,
                                },
                            })],
                            depth_stencil_attachment: None,
                        });

                        pass.set_pipeline(&pipeline);
                        pass.set_bind_group(0, &camera_bind_group, &[]);

                        if let Some(incoming_boxes) = requested_drawables.lock().unwrap().take() {
                            quads.clear();

                            quads.append(&mut render_drawables(incoming_boxes, device))
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
}

fn render_drawables(drawables: Vec<Drawable>, device: &Device) -> Vec<Arc<Quads>> {
    drawables
        .into_iter()
        .flat_map(|drawable| match drawable {
            Drawable::Quad(q) => vec![q],
            Drawable::SubTree {
                children,
                size: _,
                transform: _,
            } => render_drawables(children.lock_ref().to_vec(), device),
            Drawable::ChildList(children) => render_drawables(children, device),
        })
        .collect()
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
