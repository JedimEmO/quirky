mod drawable_tree_watch;
pub mod drawables;
pub mod quirky_app_context;
mod ui_camera;
pub mod view_tree;
pub mod widget;
pub mod widgets;
pub mod primitives;

use async_std::task::{block_on, sleep};
use std::fmt::Debug;
use std::iter;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use futures::{select, Stream};
use futures::{Future, FutureExt};
use futures_signals::map_ref;
use futures_signals::signal::{Mutable, Signal, SignalExt};
use futures_signals::signal_vec::MutableVec;
use futures_signals::signal_vec::SignalVecExt;

use crate::drawable_tree_watch::drawable_tree_watch;
use crate::drawables::Drawable;
use crate::ui_camera::UiCamera2D;
use glam::UVec2;
use glyphon::{FontSystem, SwashCache, TextAtlas};
use quirky_app_context::QuirkyAppContext;
use uuid::Uuid;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferUsages, Device, Queue,
    ShaderStages, Surface, TextureFormat,
};
use widget::Widget;
use crate::primitives::{DrawablePrimitive, Primitive, RenderContext};
use crate::quirky_app_context::FontContext;

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

#[derive(Clone)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Num(usize),
}

#[derive(Clone)]
pub enum MouseEvent {
    Enter { pos: UVec2 },
    Leave {},
    Move { pos: UVec2 },
    ButtonDown { button: MouseButton },
    ButtonUp { button: MouseButton },
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
    requested_drawables: Arc<Mutex<Option<Vec<Drawable>>>>,
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

        let context = QuirkyAppContext::new(
            device,
            queue,
            FontContext {
                font_system: font_system.into(),
                font_cache: font_cache.into(),
                text_atlas: text_atlas.into(),
            },
            viewport_size.read_only(),
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
            requested_drawables: Arc::new(Mutex::new(None)),
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
        let (drawables, fut) = run_widgets(&self.context, widgets, &*self.context.device);
        let (out, drawables_watch_fut) = drawable_tree_watch(drawables.clone());

        let mut run_futs = FuturesUnordered::new();
        let requested_drawables = self.requested_drawables.clone();
        let out =
            futures_signals::signal::from_stream(out).throttle(|| sleep(Duration::from_millis(10)));

        run_futs.push(drawables_watch_fut.boxed());
        run_futs.push(fut.boxed());
        run_futs.push(clone!(
            requested_drawables,
            clone!(
                drawables,
                out.for_each(move |_| {
                    let _ = requested_drawables
                        .lock()
                        .unwrap()
                        .insert(drawables.lock_ref().to_vec());

                    on_new_drawables();

                    async move {}
                })
                .boxed()
            )
        ));

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

            let mut encoder = self
                .context.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("hi there"),
                });

            let mut drawables: Vec<Arc<dyn DrawablePrimitive + Send + Sync>> = vec![];

            let incoming_drawables = self.requested_drawables.lock().unwrap().take();

            if let Some(incoming_drawables) = incoming_drawables {
                let text_atlas = self.context.font_context.text_atlas.lock().unwrap();

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

                // pass.set_bind_group(0, &self.camera_bind_group, &[]);
                drawables.clear();
                drawables.append(&mut render_drawables(incoming_drawables));

                {
                    let render_context = RenderContext { text_atlas: &*text_atlas, camera_bind_group: &self.camera_bind_group, screen_resolution };

                    drawables.iter().for_each(|d| {
                        d.draw(&mut pass, &render_context);
                    });
                }
            }

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

fn render_drawables(drawables: Vec<Drawable>) -> Vec<Arc<dyn DrawablePrimitive + Send + Sync>> {
    drawables
        .into_iter()
        .flat_map(|drawable| match drawable {
            Drawable::Quad(q) => vec![q as Arc<dyn DrawablePrimitive + Send + Sync>],
            Drawable::SubTree {
                children,
                size: _,
                transform: _,
            } => render_drawables(children.lock_ref().to_vec()),
            Drawable::ChildList(children) => render_drawables(children),
            Drawable::Primitive(primitive) => { vec![primitive] }
        })
        .collect()
}

pub fn run_widgets<'a, 'b: 'a>(
    ctx: &'b QuirkyAppContext,
    widgets: MutableVec<Arc<dyn Widget>>,
    device: &'a Device,
) -> (MutableVec<Drawable>, impl Future<Output=()> + 'a) {
    let data = MutableVec::new();

    let runner_fut = clone!(data, async move {
        let next_widgets_stream = widgets.signal_vec_cloned().to_signal_cloned().to_stream();

        let mut next_widgets_stream = next_widgets_stream.map(clone!(data, move |v| {
            let futures = FuturesUnordered::new();

            for (idx, widget) in v.into_iter().enumerate() {
                let bb = widget.bounding_box().get();
                let subtree_data = MutableVec::new();

                futures.push(widget.run(ctx, subtree_data.clone(), device));

                let mut d = data.lock_mut();

                let drawable = Drawable::SubTree {
                    children: subtree_data,
                    transform: bb.pos,
                    size: bb.size,
                };

                if idx < d.len() {
                    d.set_cloned(idx, drawable);
                } else {
                    d.push_cloned(drawable)
                }
            }

            futures
        }));

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
    });

    (data, runner_fut)
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
    container_box: impl Signal<Item=LayoutBox> + Send,
    constraints: impl Signal<Item=Vec<Box<dyn Signal<Item=SizeConstraint> + Unpin + Send>>> + Send,
    extras_signal: impl Signal<Item=TExtras> + Send,
    layout_strategy: impl Fn(&LayoutBox, &Vec<SizeConstraint>, &TExtras) -> Vec<LayoutBox> + Send,
) -> impl Signal<Item=Vec<LayoutBox>> + Send {
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
        return self.widget.get_widget_at(pos, vec![]);
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

#[cfg(test)]
mod t {
    use crate::quirky_app_context::QuirkyAppContext;
    use crate::run_widgets;
    use crate::widget::Widget;
    use crate::widgets::slab::SlabBuilder;
    use futures_signals::signal_vec::MutableVec;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::time::sleep;
    use wgpu::{DownlevelCapabilities, Features, Limits};
    use wgpu_test::{initialize_test, TestParameters};

    #[tokio::test]
    async fn test_run_widgets() {
        let device = Arc::new(Mutex::new(None));

        clone!(
            device,
            initialize_test(
                TestParameters {
                    failures: vec![],
                    required_downlevel_properties: DownlevelCapabilities::default(),
                    required_limits: Limits::default(),
                    required_features: Features::default()
                },
                |ctx| {
                    let _ = device.lock().unwrap().insert(Some(ctx));
                }
            )
        );

        let ctx = Box::leak(Box::new(
            device.lock().unwrap().take().unwrap().take().unwrap(),
        ));

        let widget: Arc<dyn Widget> = SlabBuilder::new().build();
        let widget2: Arc<dyn Widget> = SlabBuilder::new().build();

        let widgets = MutableVec::new_with_values(vec![widget.clone()]);
        let qctx = Box::leak(Box::new(QuirkyAppContext::new()));
        let (drawables, fut) = run_widgets(qctx, widgets.clone(), &ctx.device);

        let _j = tokio::spawn(fut);

        sleep(Duration::from_millis(100)).await;
        assert_eq!(drawables.lock_ref().len(), 1);

        widgets.lock_mut().push_cloned(widget2.clone());

        sleep(Duration::from_millis(100)).await;

        assert_eq!(drawables.lock_ref().len(), 2);
    }
}
