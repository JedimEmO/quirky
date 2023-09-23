pub mod primitives;
pub mod quirky_app_context;
pub mod styling;
mod ui_camera;
pub mod view_tree;
pub mod widget;
pub mod widgets;

use crate::primitives::{DrawablePrimitive, Primitive, RenderContext};
use crate::quirky_app_context::FontContext;
use crate::ui_camera::UiCamera2D;
use async_std::task::{block_on, sleep};
use futures::select;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use futures::{Future, FutureExt};
use futures_signals::map_ref;
use futures_signals::signal::{Mutable, Signal, SignalExt};
use futures_signals::signal_vec::MutableVec;
use futures_signals::signal_vec::SignalVecExt;
use glam::UVec2;
use glyphon::{FontSystem, SwashCache, TextAtlas};
use primitives::PrepareContext;
use quirky_app_context::QuirkyAppContext;
use std::borrow::BorrowMut;
use std::fmt::Debug;
use std::iter;
use std::sync::atomic::{AtomicI64, Ordering};
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

#[macro_export]
macro_rules! clone {
    ($v:ident, $b:expr) => {{
        let $v = $v.clone();
        ($b)
    }};
}

pub struct LayoutToken {
    layout_counter: Arc<AtomicI64>,
}

impl LayoutToken {
    pub fn new(counter: Arc<AtomicI64>) -> Self {
        counter.fetch_add(1, Ordering::Relaxed);
        Self {
            layout_counter: counter,
        }
    }
}

impl Drop for LayoutToken {
    fn drop(&mut self) {
        self.layout_counter.fetch_add(-1, Ordering::Relaxed);
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Num(usize),
}

#[derive(Clone)]
pub enum MouseEvent {
    Enter {
        pos: UVec2,
    },
    Leave {},
    Move {
        pos: UVec2,
    },
    ButtonDown {
        button: MouseButton,
    },
    ButtonUp {
        button: MouseButton,
    },
    Drag {
        from: UVec2,
        to: UVec2,
        button: MouseButton,
    },
}

#[derive(Clone)]
pub enum WidgetEvent {
    MouseEvent { event: MouseEvent },
}

#[derive(Clone)]
pub struct EventDispatch {
    pub receiver_id: Uuid,
    pub event: WidgetEvent,
}

pub struct QuirkyApp {
    pub context: QuirkyAppContext,
    pub viewport_size: Mutable<UVec2>,
    pub widget: Arc<dyn Widget>,
    ui_camera: Mutex<UiCamera2D>,
    surface_format: TextureFormat,
    camera_uniform_buffer: Buffer,
    camera_bind_group_layout: BindGroupLayout,
    camera_bind_group: BindGroup,
    signal_dirty_rx: async_std::channel::Receiver<()>,
    drawables_cache: Mutex<Vec<(Uuid, Vec<Box<dyn DrawablePrimitive>>)>>,
}

impl QuirkyApp {
    pub fn new(
        device: Device,
        queue: Queue,
        surface_format: TextureFormat,
        widget: Arc<dyn Widget>,
        font_system: FontSystem,
        font_cache: SwashCache,
    ) -> Self {
        let viewport_size = Mutable::new(Default::default());
        let ui_camera = UiCamera2D::default();
        let (camera_buffer, camera_bind_group_layout, camera_bind_group) =
            Self::setup(&device, &ui_camera);

        let text_atlas = TextAtlas::new(&device, &queue, surface_format);
        let (tx, rx) = async_std::channel::unbounded();

        let context = QuirkyAppContext::new(
            device,
            queue,
            FontContext {
                font_system: font_system.into(),
                font_cache: font_cache.into(),
                text_atlas: text_atlas.into(),
            },
            viewport_size.read_only(),
            tx,
        );

        Self {
            context,
            viewport_size,
            widget,
            ui_camera: ui_camera.into(),
            surface_format,
            camera_uniform_buffer: camera_buffer,
            camera_bind_group_layout,
            camera_bind_group,
            signal_dirty_rx: rx,
            drawables_cache: Default::default(),
        }
    }

    pub fn configure_primitive<T: Primitive>(&self) {
        T::configure_pipeline(
            &self.context.device,
            &[&self.camera_bind_group_layout],
            self.surface_format,
        );
    }

    pub async fn run(self: Arc<Self>, on_new_drawables: impl Fn() + Send) {
        let widgets = MutableVec::new_with_values(vec![self.widget.clone()]);
        let fut = run_widgets(&self.context, widgets);

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

            let mut drawables_cache = self.drawables_cache.lock().unwrap();

            let mut font_system = block_on(self.context.font_context.font_system.lock());
            let mut font_cache = block_on(self.context.font_context.font_cache.lock());
            let mut text_atlas = block_on(self.context.font_context.text_atlas.lock());

            let mut pipeline_cache = Default::default();
            let mut bind_group_cache = Default::default();

            let mut paint_context = PrepareContext {
                font_system: font_system.borrow_mut(),
                text_atlas: text_atlas.borrow_mut(),
                font_cache: font_cache.borrow_mut(),
                device: &self.context.device,
                queue: &self.context.queue,
                surface_format: self.surface_format,
                pipeline_cache: &mut pipeline_cache,
                bind_group_cache: &mut bind_group_cache,
                camera_bind_group_layout: &self.camera_bind_group_layout,
            };

            let mut drawables = next_drawable_list(
                &self.widget,
                &mut drawables_cache,
                &self.context,
                &mut paint_context,
            );

            for drawable in drawables.iter_mut() {
                for d in drawable.1.iter_mut() {
                    d.prepare(&mut paint_context);
                }
            }

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

                let render_context = RenderContext {
                    text_atlas: &text_atlas,
                    camera_bind_group: &self.camera_bind_group,
                    screen_resolution,
                    pipeline_cache: &pipeline_cache,
                    bind_group_cache: &bind_group_cache,
                };

                drawables.iter().for_each(|d| {
                    d.1.iter().for_each(|d2| {
                        d2.draw(&mut pass, &render_context);
                    });
                });
            }

            drawables_cache.clear();
            drawables_cache.append(&mut drawables);

            self.context.queue.submit(iter::once(encoder.finish()));

            output.present();
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

pub fn run_widgets<'a, 'b: 'a>(
    ctx: &'b QuirkyAppContext,
    widgets: MutableVec<Arc<dyn Widget>>,
) -> impl Future<Output = ()> + 'a {
    

    async move {
        let next_widgets_stream = widgets.signal_vec_cloned().to_signal_cloned().to_stream();

        let mut next_widgets_stream = next_widgets_stream.map(move |v| {
            let futures = FuturesUnordered::new();

            for (_idx, widget) in v.into_iter().enumerate() {
                futures.push(widget.run(ctx));
            }

            futures
        });

        let mut updates = FuturesUnordered::new();

        loop {
            let mut ws_next = next_widgets_stream.next().fuse();
            let mut updated_next = updates.next().fuse();

            select! {
                nws = ws_next => {
                    if let Some(nws) = nws {
                        updates = nws;
                    }
                }
                _un = updated_next => {
                }
            }
        }
    }
}

fn next_drawable_list(
    widget: &Arc<dyn Widget>,
    old_list: &mut Vec<(Uuid, Vec<Box<dyn DrawablePrimitive>>)>,
    ctx: &QuirkyAppContext,
    paint_ctx: &mut PrepareContext,
) -> Vec<(Uuid, Vec<Box<dyn DrawablePrimitive>>)> {
    let widget_id = widget.id();
    let widget_primitives = if widget.dirty().get() {
        widget.clear_dirty();
        vec![(widget_id, widget.paint(ctx, paint_ctx))]
    } else if let Some(old_index) = old_list.iter().position(|v| v.0 == widget_id) {
        vec![old_list.remove(old_index)]
    } else {
        vec![]
    };

    let child_primitives = widget
        .children()
        .map(|children| {
            children
                .iter()
                .flat_map(|child| next_drawable_list(child, old_list, ctx, paint_ctx))
                .collect::<Vec<_>>()
        })
        .or(Some(vec![]))
        .unwrap();

    vec![widget_primitives, child_primitives]
        .into_iter()
        .flatten()
        .collect()
}

#[derive(Default, Clone, Copy, Debug)]
pub enum SizeConstraint {
    MinSize(UVec2),
    #[default]
    Unconstrained,
    MaxHeight(u32),
    MaxWidth(u32),
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

pub fn layout<TExtras: Send>(
    container_box: impl Signal<Item = LayoutBox> + Send,
    constraints: impl Signal<Item = Vec<Box<dyn Signal<Item = SizeConstraint> + Unpin + Send>>> + Send,
    extras_signal: impl Signal<Item = TExtras> + Send,
    layout_strategy: impl Fn(&LayoutBox, &Vec<SizeConstraint>, &TExtras) -> Vec<LayoutBox> + Send,
) -> impl Signal<Item = Vec<LayoutBox>> + Send {
    let constraints = constraints.to_signal_vec();
    let constraints = constraints.map_signal(|x| x).to_signal_cloned();

    map_ref! {
        let container_box = container_box,
        let child_constraints = constraints,
        let extras = extras_signal => {
            layout_strategy(container_box, child_constraints, extras)
        }
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
