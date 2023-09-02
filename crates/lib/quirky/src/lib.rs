pub mod view_tree;

use std::fmt::Debug;
use std::sync::Arc;


use crate::drawables::Drawable;
use futures::select;
use futures::Stream;
use futures::StreamExt;
use futures::{Future, FutureExt};
use futures::stream::FuturesUnordered;
use futures_signals::map_ref;
use futures_signals::signal::{always, Mutable, Signal, SignalExt};
use futures_signals::signal_vec::MutableVec;
use futures_signals::signal_vec::SignalVecExt;

use glam::UVec2;

#[derive(Clone)]
pub struct CompoundNode {
    pub(crate) bounding_box: Mutable<LayoutBox>,
    pub(crate) children: Mutable<Vec<CompoundNode>>,
}

macro_rules! clone {
    ($v:ident, $b:expr) => {{
        let $v = $v.clone();
        ($b)
    }};
}

pub fn run_widgets(
    widgets: MutableVec<Arc<dyn Widget>>,
) -> (MutableVec<Drawable>, impl Future<Output=()>) {
    let data = MutableVec::new();

    let runner_fut = clone!(data, async move {
        let next_widgets_stream = widgets.signal_vec_cloned().to_signal_cloned().to_stream();

        let mut next_widgets_stream = next_widgets_stream.map(clone!(data, move |v| {
            let futures = FuturesUnordered::new();

            for (idx, widget) in v.into_iter().enumerate() {
                let subtree_data = MutableVec::new();
                futures.push(widget.run(subtree_data.clone()));

                let mut d = data.lock_mut();

                if idx < d.len() {
                    d.set_cloned(idx, Drawable::SubTree(subtree_data));
                } else {
                    d.push_cloned(Drawable::SubTree(subtree_data))
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
    use std::sync::Arc;
    use std::time::Duration;
    use futures_signals::signal_vec::MutableVec;
    use tokio::time::sleep;
    use crate::widgets::List;
    use crate::{run_widgets, Widget};

    #[tokio::test]
    async fn test_run_widgets() {
        let widget: Arc<dyn Widget> = Arc::new(List::default());
        let widget2: Arc<dyn Widget> = Arc::new(List::default());

        let widgets = MutableVec::new_with_values(vec![widget.clone()]);
        let (drawables, fut) = run_widgets(widgets.clone());

        let _j = tokio::spawn(fut);

        sleep(Duration::from_millis(100)).await;
        assert_eq!(drawables.lock_ref().len(), 1);

        widgets.lock_mut().push_cloned(widget2.clone());

        sleep(Duration::from_millis(100)).await;

        assert_eq!(drawables.lock_ref().len(), 2);
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub enum SizeConstraint {
    MinSize(UVec2),
    #[default]
    Unconstrained,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct LayoutBox {
    pub pos: UVec2,
    pub size: UVec2,
}

pub mod drawables {
    use futures_signals::signal_vec::MutableVec;
    use glam::UVec2;

    #[derive(Clone, Debug)]
    pub enum Drawable {
        Quad { pos: UVec2, size: UVec2 },
        ChildList(Vec<Drawable>),
        SubTree(MutableVec<Drawable>),
    }
}

#[async_trait::async_trait]
pub trait Widget: Sync + Send + Debug {
    fn paint(&self) -> Vec<Drawable>;
    fn size_constraint(&self) -> Box<dyn Signal<Item=SizeConstraint> + Unpin + Send>;

    async fn run(self: Arc<Self>, drawable_data: MutableVec<Drawable>);
}

pub fn layout(
    container_box: impl Signal<Item=LayoutBox> + Send,
    constraints: impl Signal<Item=Vec<Box<dyn Signal<Item=SizeConstraint> + Unpin + Send>>> + Send,
    layout_strategy: impl Fn(&LayoutBox, &Vec<SizeConstraint>) -> Vec<LayoutBox> + Send,
) -> impl Signal<Item=Vec<LayoutBox>> + Send {
    let constraints = constraints.to_signal_vec();
    let constraints = constraints.map_signal(|x| x).to_signal_cloned();

    map_ref! {
        let container_box = container_box,
        let child_constraints = constraints => {
            layout_strategy(container_box, child_constraints)
        }
    }
}

pub mod widgets {
    use crate::drawables::Drawable;
    
    use crate::{layout, LayoutBox, SizeConstraint, Widget};
    use async_trait::async_trait;
    use futures::select;
    use futures::{FutureExt, StreamExt};
    use futures_signals::signal::{always, Mutable, Signal, SignalExt};
    use futures_signals::signal_vec::{MutableVec, SignalVecExt};
    use glam::{uvec2, UVec2};
    use std::sync::Arc;

    #[derive(Debug)]
    pub struct Slab {}

    #[async_trait]
    impl Widget for Slab {
        fn paint(&self) -> Vec<Drawable> {
            vec![Drawable::Quad {
                pos: uvec2(0, 0),
                size: uvec2(10, 10),
            }]
        }

        fn size_constraint(&self) -> Box<dyn Signal<Item=SizeConstraint> + Unpin + Send> {
            Box::new(always(SizeConstraint::MinSize(uvec2(10, 10))))
        }

        async fn run(self: Arc<Self>, _drawable_data: MutableVec<Drawable>) {
            loop {
                async {}.await;
            }
        }
    }

    #[derive(Default, Debug)]
    pub struct List {
        pub children: Mutable<Vec<Arc<dyn Widget>>>,
        pub requested_size: Mutable<SizeConstraint>,
    }

    #[async_trait]
    impl Widget for List {
        fn paint(&self) -> Vec<Drawable> {
            vec![]
        }

        fn size_constraint(&self) -> Box<dyn Signal<Item=SizeConstraint> + Unpin + Send> {
            Box::new(self.requested_size.signal_cloned())
        }

        async fn run(self: Arc<Self>, drawable_data: MutableVec<Drawable>) {
            let child_layouts = layout(
                self.requested_size.signal_cloned().map(|_s| LayoutBox { pos: UVec2::new(0, 0), size: UVec2::new(100, 500) }),
                self.children.signal_cloned().map(|v| v.into_iter().map(|c| c.size_constraint()).collect()),
                |a, b| {
                    let item_heights = a.size.y / b.len().max(1) as u32;

                    (0..b.len()).map(|i| {
                        LayoutBox { pos: a.pos + UVec2::new(0, (i * item_heights as usize + i * 5) as u32), size: UVec2::new(100, item_heights) }
                    }).collect()
                },
            );

            let mut child_layouts_stream = child_layouts.to_stream();

            loop {
                let mut next_layouts = child_layouts_stream.next().fuse();

                select! {
                    layouts = next_layouts => {
                        drawable_data.lock_mut().replace_cloned(layouts.into_iter().map(|v: Vec<LayoutBox>| {
                            Drawable::ChildList(v.into_iter().map(|v| Drawable::Quad { pos: v.pos, size: v.size }).collect::<Vec<_>>())
                        }).collect::<Vec<_>>());
                    }
                }
            }
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
    container_box: impl Signal<Item=LayoutBox>,
    strategy: impl Signal<Item=VerticalLayoutStrategy>,
    requested_item_count: impl Signal<Item=usize>,
) -> impl Signal<Item=(LayoutBox, Vec<LayoutBox>)> {
    map_ref! {
        let container_box = container_box,
        let strategy = strategy,
        let requested_item_count = requested_item_count => {
            let item_height = match strategy {
                VerticalLayoutStrategy::BoxHeight { height } => { requested_item_count * height }
                VerticalLayoutStrategy::Even => { if *requested_item_count == 0 {0} else { container_box.size.y as usize / requested_item_count }}
            };

            (LayoutBox {
                pos: container_box.pos,
                size: UVec2::new(container_box.size.x, (item_height * requested_item_count) as u32),
            }, (0..*requested_item_count).map(|i| {
                LayoutBox {
                    pos: UVec2::new(0, (i * item_height) as u32),
                    size: UVec2::new(container_box.size.x, item_height as u32),
                }
            }).collect::<Vec<_>>())
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
fn quirky_ui(_view: impl Signal<Item=ViewBuffer>) -> impl Signal<Item=ViewBuffer> {
    always(ViewBuffer {})
}

fn button(_props: (), _events: impl Stream<Item=WidgetEvents>) -> impl Signal<Item=ViewBuffer> {
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
