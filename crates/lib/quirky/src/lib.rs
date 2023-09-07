pub mod primitives;
pub mod view_tree;
pub mod widget;
pub mod widgets;

use std::fmt::Debug;
use std::sync::Arc;

use crate::drawables::Drawable;
use futures::select;
use futures::stream::FuturesUnordered;
use futures::Stream;
use futures::StreamExt;
use futures::{Future, FutureExt};
use futures_signals::map_ref;
use futures_signals::signal::{always, Mutable, Signal, SignalExt};
use futures_signals::signal_vec::MutableVec;
use futures_signals::signal_vec::SignalVecExt;

use glam::UVec2;
use wgpu::Device;
use widget::Widget;

#[derive(Clone)]
pub struct CompoundNode {
    pub(crate) bounding_box: Mutable<LayoutBox>,
    pub(crate) children: Mutable<Vec<CompoundNode>>,
}

#[macro_export]
macro_rules! clone {
    ($v:ident, $b:expr) => {{
        let $v = $v.clone();
        ($b)
    }};
}

pub fn run_widgets(
    widgets: MutableVec<Arc<dyn Widget>>,
    device: &Device,
) -> (MutableVec<Drawable>, impl Future<Output = ()> + '_) {
    let data = MutableVec::new();

    let runner_fut = clone!(data, async move {
        let next_widgets_stream = widgets.signal_vec_cloned().to_signal_cloned().to_stream();

        let mut next_widgets_stream = next_widgets_stream.map(clone!(data, move |v| {
            let futures = FuturesUnordered::new();

            for (idx, widget) in v.into_iter().enumerate() {
                let bb = widget.bounding_box().get();
                let subtree_data = MutableVec::new();

                futures.push(widget.run(subtree_data.clone(), device));

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

#[derive(Default, Clone, Copy, Debug)]
pub enum SizeConstraint {
    MinSize(UVec2),
    #[default]
    Unconstrained,
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

pub fn layout(
    container_box: impl Signal<Item = LayoutBox> + Send,
    constraints: impl Signal<Item = Vec<Box<dyn Signal<Item = SizeConstraint> + Unpin + Send>>> + Send,
    layout_strategy: impl Fn(&LayoutBox, &Vec<SizeConstraint>) -> Vec<LayoutBox> + Send,
) -> impl Signal<Item = Vec<LayoutBox>> + Send {
    let constraints = constraints.to_signal_vec();
    let constraints = constraints.map_signal(|x| x).to_signal_cloned();

    map_ref! {
        let container_box = container_box,
        let child_constraints = constraints => {
            layout_strategy(container_box, child_constraints)
        }
    }
}

pub struct ViewBuffer {}

pub enum WidgetEvents {
    NewBuffer(()),
}

pub enum VerticalLayoutStrategy {
    Even,
    BoxHeight { height: usize },
}

fn lay_out_boxes_vertically(
    container_box: impl Signal<Item = LayoutBox>,
    strategy: impl Signal<Item = VerticalLayoutStrategy>,
    requested_item_count: impl Signal<Item = usize>,
) -> impl Signal<Item = Vec<LayoutBox>> {
    map_ref! {
        let container_box = container_box,
        let strategy = strategy,
        let requested_item_count = requested_item_count => {
            let item_height = match strategy {
                VerticalLayoutStrategy::BoxHeight { height } => { requested_item_count * height }
                VerticalLayoutStrategy::Even => { if *requested_item_count == 0 {0} else { container_box.size.y as usize / requested_item_count }}
            };

            (0..*requested_item_count).map(|i| {
                LayoutBox {
                    pos: UVec2::new(0, (i * item_height) as u32),
                    size: UVec2::new(container_box.size.x, item_height as u32),
                }
            }).collect::<Vec<_>>()
        }
    }
}
//
// pub trait ListItem: Widget + Clone + 'static {}
//
// fn list<TItem: Widget + Clone>(
//     box_size: impl Signal<Item=LayoutBox>,
//     items: MutableVec<TItem>,
// ) -> impl Signal<Item=Vec<Drawable>> {
//     let layout = lay_out_boxes_vertically(
//         box_size,
//         always(VerticalLayoutStrategy::Even),
//         items.signal_vec_cloned().len(),
//     );
//
//     layout.map(move |(container_box, item_boxes)| {
//         // items.lock_ref().iter().zip(item_boxes).map(|item| item.0.paint())
//         Drawable::Quad { pos: container_box.pos, size: container_box.size }
//     })
// }

fn canvas_layout() {}

/// Top level of any quirky UI window
fn quirky_ui(_view: impl Signal<Item = ViewBuffer>) -> impl Signal<Item = ViewBuffer> {
    always(ViewBuffer {})
}

fn button(_props: (), _events: impl Stream<Item = WidgetEvents>) -> impl Signal<Item = ViewBuffer> {
    always(ViewBuffer {})
}

fn my_main() {
    let _ = quirky_ui(button(
        (),
        futures::stream::iter([WidgetEvents::NewBuffer(())]),
    ));
}

#[cfg(test)]
mod test {

    // #[tokio::test]
    // async fn test_list_layout() {
    //     let layout = lay_out_boxes_vertically(
    //         always(LayoutBox {
    //             pos: UVec2::new(10, 10),
    //             size: UVec2::new(100, 400),
    //         }),
    //         always(VerticalLayoutStrategy::Even),
    //         always(4),
    //     );
    //
    //     let next_list_box = layout.to_future().await;
    //     assert_eq!(
    //         next_list_box.0,
    //         LayoutBox {
    //             pos: UVec2::new(10, 10),
    //             size: UVec2::new(100, 400),
    //         }
    //     );
    // }

    #[test]
    fn list_test() {}
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
