pub mod drawable_primitive;
pub mod quirky_app_context;
pub mod render_contexts;
mod ui_camera;
pub mod widget;
pub mod widgets;

use crate::quirky_app_context::QuirkyResources;
use crate::ui_camera::UiCamera2D;
use async_std::task::sleep;
use drawable_primitive::DrawablePrimitive;
use futures::stream::FuturesUnordered;
use futures::FutureExt;
use futures::StreamExt;
use futures_signals::signal::{Mutable, SignalExt};
use futures_signals::signal_vec::MutableVec;
use glam::UVec2;
use quirky_app_context::QuirkyAppContext;
use render_contexts::PrepareContext;
use render_contexts::RenderContext;
use std::borrow::BorrowMut;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::iter;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferUsages, Device, Queue,
    ShaderStages, Surface, TextureFormat,
};
use widget::Widget;
use widgets::events::WidgetEvent;
use widgets::run_widget;

#[macro_export]
macro_rules! clone {
    ($v:ident, $b:expr) => {{
        let $v = $v.clone();
        ($b)
    }};
}

pub struct QuirkyApp {
    pub context: QuirkyAppContext,
    pub viewport_size: Mutable<UVec2>,
    pub widget: Arc<dyn Widget>,
    pub resources: Mutex<QuirkyResources>,
    ui_camera: Mutex<UiCamera2D>,
    surface_format: TextureFormat,
    camera_uniform_buffer: Buffer,
    camera_bind_group_layout: BindGroupLayout,
    camera_bind_group: BindGroup,
    signal_dirty_rx: async_std::channel::Receiver<()>,
}

impl QuirkyApp {
    pub fn new(
        device: Device,
        queue: Queue,
        surface_format: TextureFormat,
        widget: Arc<dyn Widget>,
    ) -> Self {
        let viewport_size = Mutable::new(Default::default());
        let ui_camera = UiCamera2D::default();
        let (camera_buffer, camera_bind_group_layout, camera_bind_group) =
            Self::setup(&device, &ui_camera);

        let (tx, rx) = async_std::channel::unbounded();

        let context = QuirkyAppContext::new(device, queue, viewport_size.read_only(), tx);

        Self {
            context,
            viewport_size,
            widget,
            resources: Mutex::new(Default::default()),
            ui_camera: ui_camera.into(),
            surface_format,
            camera_uniform_buffer: camera_buffer,
            camera_bind_group_layout,
            camera_bind_group,
            signal_dirty_rx: rx,
        }
    }

    pub async fn run(self: Arc<Self>, on_new_drawables: impl Fn() + Send) {
        let widgets = MutableVec::new_with_values(vec![self.widget.clone()]);
        let fut = run_widget::run_widgets(&self.context, widgets.signal_vec_cloned());

        let mut run_futs = FuturesUnordered::new();

        run_futs.push(fut.boxed());
        run_futs.push(
            futures_signals::signal::from_stream(self.signal_dirty_rx.clone())
                .throttle(|| sleep(Duration::from_millis(16)))
                .for_each(move |_| {
                    on_new_drawables();
                    async move {}
                })
                .boxed(),
        );

        run_futs.push(
            self.viewport_size
                .signal()
                .throttle(|| async_std::task::sleep(Duration::from_millis(5)))
                .for_each(|new_viewport_size| {
                    self.ui_camera
                        .lock()
                        .unwrap()
                        .resize_viewport(new_viewport_size);

                    self.widget.set_bounding_box(LayoutBox {
                        pos: Default::default(),
                        size: new_viewport_size,
                    });

                    async move {}
                })
                .boxed(),
        );

        loop {
            run_futs.select_next_some().await;
        }
    }

    pub fn draw(&self, surface: &Surface) -> anyhow::Result<()> {
        let camera_uniform = self.ui_camera.lock().unwrap().create_camera_uniform();
        let screen_resolution = self.context.viewport_size.get();

        self.context.queue.write_buffer(
            &self.camera_uniform_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );

        if let Ok(output) = surface.get_current_texture().map_err(|e| {
            println!("surface error: {:?}", e);
            e
        }) {
            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            let mut encoder =
                self.context
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("hi there"),
                    });

            let mut pipeline_cache = Default::default();
            let mut bind_group_cache = Default::default();
            let mut resources = self.resources.lock().unwrap();
            let mut out_list = VecDeque::with_capacity(10000);

            {
                let mut paint_context = PrepareContext {
                    resources: resources.borrow_mut(),
                    device: &self.context.device,
                    queue: &self.context.queue,
                    surface_format: self.surface_format,
                    pipeline_cache: &mut pipeline_cache,
                    bind_group_cache: &mut bind_group_cache,
                    camera_bind_group_layout: &self.camera_bind_group_layout,
                };

                next_drawable_list(
                    &self.widget,
                    &self.context,
                    &mut paint_context,
                    &mut out_list,
                );

                for drawable in out_list.iter_mut() {
                    for d in drawable.1.iter_mut() {
                        d.prepare(&mut paint_context);
                    }
                }
            }
            let render_context = RenderContext {
                resources: resources.borrow_mut(),
                camera_bind_group: &self.camera_bind_group,
                screen_resolution,
                pipeline_cache: &pipeline_cache,
                bind_group_cache: &bind_group_cache,
            };

            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            }),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });

                out_list.iter().for_each(|d| {
                    d.1.iter().for_each(|d2| {
                        d2.draw(&mut pass, &render_context);
                    });
                });
            }

            self.context.queue.submit(iter::once(encoder.finish()));

            output.present();

            for d in out_list {
                d.2.set_cached_primitives(Some(d.1));
            }
        }

        Ok(())
    }

    fn setup(device: &Device, camera: &UiCamera2D) -> (Buffer, BindGroupLayout, BindGroup) {
        let camera_uniform = camera.create_camera_uniform();

        let camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("camera buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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

        (camera_buffer, camera_bind_group_layout, camera_bind_group)
    }
}

fn next_drawable_list(
    widget: &Arc<dyn Widget>,
    ctx: &QuirkyAppContext,
    paint_ctx: &mut PrepareContext,
    out: &mut VecDeque<(Uuid, Vec<Box<dyn DrawablePrimitive>>, Arc<dyn Widget>)>,
) {
    let widget_id = widget.id();

    if widget.dirty().get() {
        widget.clear_dirty();
        out.push_back((widget_id, widget.prepare(ctx, paint_ctx), widget.clone()));
    } else {
        out.push_back((
            widget_id,
            widget.get_cached_primitives().or(Some(vec![])).unwrap(),
            widget.clone(),
        ));
    }

    widget.children().map(|v| {
        v.iter()
            .for_each(|child| next_drawable_list(child, ctx, paint_ctx, out))
    });
}

#[derive(PartialEq, Clone, Copy, Debug, Default)]
pub struct LayoutBox {
    pub pos: UVec2,
    pub size: UVec2,
}

impl LayoutBox {
    pub fn contains(&self, pos: UVec2) -> bool {
        let br = self.pos + self.size;

        pos.x >= self.pos.x && pos.y >= self.pos.y && pos.x < br.x && pos.y < br.y
    }
}

impl QuirkyApp {
    pub fn get_widgets_at(&self, pos: UVec2) -> Option<Vec<Uuid>> {
        self.widget.get_widget_at(pos, vec![])
    }

    pub fn dispatch_event_to_widget(&self, target: Uuid, event: WidgetEvent) {
        self.context
            .dispatch_event(target, event)
            .expect("failed dispatching event");
    }
}

#[macro_export]
macro_rules! assert_f32_eq {
    ($l:expr, $r:expr, $msg:expr) => {{
        let diff = ($l - $r).abs();

        if diff > f32::EPSILON {
            panic!(
                "{} - f32 difference {} is greater than allowed diff f32::EPSILON",
                $msg, diff
            );
        }
    }};
}
