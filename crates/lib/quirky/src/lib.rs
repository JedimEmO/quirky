pub mod primitives;
pub mod quirky_app_context;
pub mod styling;
mod ui_camera;
pub mod view_tree;
pub mod widget;
pub mod widgets;

use crate::primitives::{DrawablePrimitive, RenderContext};
use crate::quirky_app_context::FontContext;
use crate::ui_camera::UiCamera2D;
use async_std::task::{block_on, sleep};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use futures::{Future, FutureExt};
use futures_signals::map_ref;
use futures_signals::signal::{Mutable, Signal, SignalExt};
use futures_signals::signal_vec::MutableVecLockMut;
use futures_signals::signal_vec::{MutableVec, SignalVec};
use futures_signals::signal_vec::{SignalVecExt, VecDiff};
use glam::UVec2;
use glyphon::{FontSystem, SwashCache, TextAtlas};
use primitives::PrepareContext;
use quirky_app_context::QuirkyAppContext;
use quirky_utils::futures_map_poll::FuturesMapPoll;
use std::borrow::BorrowMut;
use std::collections::{HashSet, VecDeque};
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

#[derive(Copy, Clone, PartialEq, Eq, Ord, PartialOrd)]
#[repr(u32)]
pub enum KeyCode {
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Escape,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    Snapshot,
    Scroll,
    Pause,
    Insert,
    Home,
    Delete,
    End,
    PageDown,
    PageUp,
    Left,
    Up,
    Right,
    Down,
    Backspace,
    Return,
    Space,
    Compose,
    Caret,
    NumLock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadDivide,
    NumpadDecimal,
    NumpadComma,
    NumpadEnter,
    NumpadEquals,
    NumpadMultiply,
    NumpadSubtract,
    Apostrophe,
    Apps,
    Asterisk,
    Backslash,
    Calculator,
    Capital,
    Colon,
    Comma,
    Period,
    Convert,
    Equals,
    Grave,
    Semicolon,
    At,
    Enter,
    Unknown,
}

#[derive(Clone, Default, Debug)]
pub struct KeyboardModifier {
    pub alt: bool,
    pub shift: bool,
    pub ctrl: bool,
}

#[derive(Clone)]
pub enum KeyboardEvent {
    KeyPressed {
        key_code: KeyCode,
        modifier: KeyboardModifier,
    },
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

#[derive(Clone, Copy, PartialEq)]
pub enum FocusState {
    Focused,
    Unfocused,
}

#[derive(Clone)]
pub enum WidgetEvent {
    KeyboardEvent { event: KeyboardEvent },
    MouseEvent { event: MouseEvent },
    FocusChange(FocusState),
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
    drawables_cache: Mutex<VecDeque<(Uuid, Vec<Box<dyn DrawablePrimitive>>)>>,
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
            drawables_cache: Mutex::new(VecDeque::with_capacity(10000)),
        }
    }

    pub async fn run(self: Arc<Self>, on_new_drawables: impl Fn() + Send) {
        let widgets = MutableVec::new_with_values(vec![self.widget.clone()]);
        let fut = run_widgets(&self.context, widgets.signal_vec_cloned());

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

            let mut out_list = VecDeque::with_capacity(10000);

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

pub fn run_widgets<'a>(
    ctx: &'a QuirkyAppContext,
    widgets_signal: impl SignalVec<Item = Arc<dyn Widget>> + Send + 'a,
) -> impl Future<Output = ()> + 'a {
    let widgets = MutableVec::new();
    let (widgets_futures_map, data) = FuturesMapPoll::new();

    let widgets_fut = widgets_signal.for_each(clone!(
        data,
        clone!(widgets, move |change: VecDiff<Arc<dyn Widget>>| {
            let mut widgets_lock = widgets.lock_mut();
            let mut widgets_futures_lock = data.lock().unwrap();

            MutableVecLockMut::<'_, _>::apply_vec_diff(&mut widgets_lock, change);

            // Add futures for newly inserted widgets
            for widget in widgets_lock.iter() {
                let id = widget.id();

                if !widgets_futures_lock.contains_key(&id) {
                    widgets_futures_lock.insert(id, widget.clone().run(ctx).boxed().into());
                }
            }

            let current_widget_ids: HashSet<Uuid> = widgets_lock.iter().map(|w| w.id()).collect();

            // Remove futures no longer in the widget list
            let ids_to_remove: Vec<Uuid> = widgets_futures_lock
                .iter()
                .filter(|w| !current_widget_ids.contains(w.0))
                .map(|w| *w.0)
                .collect();

            for id_to_remove in ids_to_remove {
                widgets_futures_lock.remove(&id_to_remove);
            }

            async move {}
        })
    ));

    let mut futs = FuturesUnordered::new();
    futs.push(widgets_fut.boxed());
    futs.push(widgets_futures_map.boxed());

    async move {
        loop {
            let _ = futs.select_next_some().await;
        }
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
        out.push_back((widget_id, widget.paint(ctx, paint_ctx), widget.clone()));
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

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
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
