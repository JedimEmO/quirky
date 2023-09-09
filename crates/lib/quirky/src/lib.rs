pub mod primitives;
pub mod view_tree;
pub mod widget;
pub mod widgets;

use std::fmt::Debug;
use std::sync::atomic::{AtomicI64,  Ordering};
use std::sync::Arc;

use crate::drawables::Drawable;
use futures::select;
use futures::stream::FuturesUnordered;
use futures::Stream;
use futures::StreamExt;
use futures::{Future, FutureExt};
use futures_signals::map_ref;
use futures_signals::signal::{Signal, SignalExt};
use futures_signals::signal_vec::MutableVec;
use futures_signals::signal_vec::SignalVecExt;

use glam::UVec2;
use wgpu::Device;
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

pub fn run_widgets<'a>(
    ctx: &'static QuirkyAppContext,
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
    MaxWidth(u32)
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
    use crate::run_widgets;
    use crate::widget::widgets::List;
    use crate::widget::Widget;
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
                    device.lock().unwrap().insert(Some(ctx));
                }
            )
        );

        let ctx = Box::leak(Box::new(
            device.lock().unwrap().take().unwrap().take().unwrap(),
        ));

        let widget: Arc<dyn Widget> = Arc::new(List::default());
        let widget2: Arc<dyn Widget> = Arc::new(List::default());

        let widgets = MutableVec::new_with_values(vec![widget.clone()]);
        let (drawables, fut) = run_widgets(widgets.clone(), &ctx.device);

        let _j = tokio::spawn(fut);

        sleep(Duration::from_millis(100)).await;
        assert_eq!(drawables.lock_ref().len(), 1);

        widgets.lock_mut().push_cloned(widget2.clone());

        sleep(Duration::from_millis(100)).await;

        assert_eq!(drawables.lock_ref().len(), 1);
    }
}