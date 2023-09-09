mod drawable_tree_watch;
pub mod primitives;
mod ui_camera;
pub mod view_tree;
pub mod widget;
pub mod widgets;

use async_std::task::{block_on, sleep};
use std::fmt::Debug;
use std::iter;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::drawables::Drawable;
use futures::select;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use futures::{Future, FutureExt};
use futures_signals::map_ref;
use futures_signals::signal::{Mutable, Signal, SignalExt};
use futures_signals::signal_vec::MutableVec;
use futures_signals::signal_vec::SignalVecExt;

use crate::drawable_tree_watch::drawable_tree_watch;
use crate::primitives::{DrawablePrimitive, Primitive};
use crate::ui_camera::UiCamera2D;
use glam::UVec2;
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

pub struct QuirkyAppContext {
    layouts_in_progress: Arc<AtomicI64>,
}

impl QuirkyAppContext {
    pub fn new() -> Self {
        Self {
            layouts_in_progress: Default::default(),
        }
    }

    pub fn start_layout(&self) -> LayoutToken {
        LayoutToken::new(self.layouts_in_progress.clone())
    }

    pub fn active_layouts(&self) -> i64 {
        self.layouts_in_progress.load(Ordering::Relaxed)
    }
}

impl Default for QuirkyAppContext {
    fn default() -> Self {
        Self::new()
    }
}

pub struct QuirkyApp {
    pub context: QuirkyAppContext,
    pub device: Device,
    pub queue: Queue,
    pub viewport_size: Mutable<UVec2>,
    pub widget: Arc<dyn Widget>,
    surface_format: TextureFormat,
    ui_camera: Mutex<UiCamera2D>,
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
    ) -> Self {
        let ui_camera = UiCamera2D::default();
        let (camera_buffer, camera_bind_group_layout, camera_bind_group) =
            Self::setup(&device, &ui_camera);

        Self {
            context: QuirkyAppContext::new(),
            device,
            queue,
            viewport_size: Default::default(),
            widget,
            surface_format,
            ui_camera: Mutex::new(ui_camera),
            camera_uniform_buffer: camera_buffer,
            camera_bind_group_layout,
            camera_bind_group,
            requested_drawables: Arc::new(Mutex::new(None)),
        }
    }

    pub fn configure_primitive<T: Primitive>(&self) {
        T::configure_pipeline(
            &self.device,
            &[&self.camera_bind_group_layout],
            self.surface_format,
        );
    }

    pub async fn run(self: Arc<Self>, on_new_drawables: impl Fn() + Send) {
        let widgets = MutableVec::new_with_values(vec![self.widget.clone()]);
        let (drawables, fut) = run_widgets(&self.context, widgets, &self.device);
        let (out, drawables_watch_fut) = drawable_tree_watch(drawables.clone());

        let mut run_futs = FuturesUnordered::new();
        let requested_drawables = self.requested_drawables.clone();

        run_futs.push(drawables_watch_fut.boxed());
        run_futs.push(fut.boxed());
        run_futs.push(clone!(
            requested_drawables,
            clone!(
                drawables,
                async move {
                    let mut out = out.fuse();

                    loop {
                        let _v = out.next().await;

                        let _ = requested_drawables
                            .lock()
                            .unwrap()
                            .insert(drawables.lock_ref().to_vec());

                        on_new_drawables();
                    }
                }
                .boxed()
            )
        ));
        run_futs.push(
            self.viewport_size
                .signal()
                // .throttle(|| async_std::task::sleep(Duration::from_millis(100)))
                .for_each(|new_viewport_size| {
                    self.ui_camera
                        .lock()
                        .unwrap()
                        .resize_viewport(new_viewport_size);

                    let camera_uniform = self.ui_camera.lock().unwrap().create_camera_uniform();

                    self.queue.write_buffer(
                        &self.camera_uniform_buffer,
                        0,
                        bytemuck::cast_slice(&[camera_uniform]),
                    );

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

    pub fn draw(&self, surface: &Surface) {
        block_on(async move {
            while self.context.active_layouts() > 0 {
                sleep(Duration::from_millis(5)).await;
            }
        });

        let output = surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("hi there"),
            });

        let mut drawables: Vec<Arc<dyn DrawablePrimitive>> = vec![];

        if let Some(incoming_drawables) = self.requested_drawables.lock().unwrap().take() {
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

            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            drawables.clear();
            drawables.append(&mut render_drawables(incoming_drawables));
            drawables.iter().for_each(|d| {
                d.draw(&mut pass);
            });
        }

        self.queue.submit(iter::once(encoder.finish()));

        output.present();
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

fn render_drawables(drawables: Vec<Drawable>) -> Vec<Arc<dyn DrawablePrimitive>> {
    drawables
        .into_iter()
        .flat_map(|drawable| match drawable {
            Drawable::Quad(q) => vec![q as Arc<dyn DrawablePrimitive>],
            Drawable::SubTree {
                children,
                size: _,
                transform: _,
            } => render_drawables(children.lock_ref().to_vec()),
            Drawable::ChildList(children) => render_drawables(children),
        })
        .collect()
}

pub fn run_widgets<'a, 'b: 'a>(
    ctx: &'b QuirkyAppContext,
    widgets: MutableVec<Arc<dyn Widget>>,
    device: &'a Device,
) -> (MutableVec<Drawable>, impl Future<Output = ()> + 'a) {
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

pub mod drawables {
    use crate::primitives::Quads;
    use futures_signals::signal_vec::MutableVec;
    use glam::UVec2;
    use std::sync::Arc;

    #[derive(Clone)]
    pub enum Drawable {
        Quad(Arc<Quads>),
        ChildList(Vec<Drawable>),
        SubTree {
            transform: UVec2,
            size: UVec2,
            children: MutableVec<Drawable>,
        },
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
    use crate::widget::widgets::List;
    use crate::widget::Widget;
    use crate::{run_widgets, QuirkyAppContext};
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

        let widget: Arc<dyn Widget> = Arc::new(List::default());
        let widget2: Arc<dyn Widget> = Arc::new(List::default());

        let widgets = MutableVec::new_with_values(vec![widget.clone()]);
        let qctx = Box::leak(Box::new(QuirkyAppContext::new()));
        let (drawables, fut) = run_widgets(qctx, widgets.clone(), &ctx.device);

        let _j = tokio::spawn(fut);

        sleep(Duration::from_millis(100)).await;
        assert_eq!(drawables.lock_ref().len(), 1);

        widgets.lock_mut().push_cloned(widget2.clone());

        sleep(Duration::from_millis(100)).await;

        assert_eq!(drawables.lock_ref().len(), 1);
    }
}
