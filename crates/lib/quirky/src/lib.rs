use futures::Stream;
use futures_signals::map_ref;
use futures_signals::signal::{always, Signal, SignalExt};
use futures_signals::signal_vec::{MutableVec, SignalVec, SignalVecExt};
use glam::UVec2;
use crate::drawables::{Drawable};

#[derive(Clone, Copy, Debug)]
pub enum SizeConstraint {
    MinSize(UVec2),
    Unconstrained,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct LayoutBox {
    pub pos: UVec2,
    pub size: UVec2,
}

pub mod drawables {
    use glam::UVec2;

    pub enum Drawable {
        Quad { pos: UVec2, size: UVec2 }
    }
}

pub trait Widget {
    fn paint(&self) -> Vec<Drawable>;
    fn size_constraint(&self) -> Box<dyn Signal<Item=SizeConstraint> + Unpin>;
}

pub fn layout(
    container_box: impl Signal<Item=LayoutBox>,
    constraints: impl Signal<Item=Vec<Box<dyn Signal<Item=SizeConstraint> + Unpin>>>,
    layout_strategy: impl Fn(&LayoutBox, &Vec<SizeConstraint>) -> Vec<LayoutBox>,
) -> impl Signal<Item=Vec<LayoutBox>> {
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
    use crate::{SizeConstraint, ViewBuffer, Widget};
    use futures_signals::signal::{always, Mutable, Signal};
    use futures_signals::signal_vec::MutableVec;
    use glam::uvec2;
    use crate::drawables::{Drawable};

    pub struct Slab {}

    impl Widget for Slab {
        fn paint(&self) -> Vec<Drawable> {
            vec![Drawable::Quad { pos: uvec2(0, 0), size: uvec2(10, 10) }]
        }

        fn size_constraint(&self) -> Box<dyn Signal<Item=SizeConstraint> + Unpin> {
            Box::new(always(SizeConstraint::MinSize(uvec2(10, 10))))
        }
    }

    pub struct List {
        pub children: MutableVec<Box<dyn Widget>>,
        pub requested_size: Mutable<SizeConstraint>,
    }


    impl Widget for List {
        fn paint(&self) -> Vec<Drawable> {
            vec![]
        }

        fn size_constraint(&self) -> Box<dyn Signal<Item=SizeConstraint> + Unpin> {
            Box::new(self.requested_size.signal_cloned())
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
fn quirky_ui(view: impl Signal<Item=ViewBuffer>) -> impl Signal<Item=ViewBuffer> {
    always(ViewBuffer {})
}

fn button(props: (), events: impl Stream<Item=WidgetEvents>) -> impl Signal<Item=ViewBuffer> {
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
    use crate::{lay_out_boxes_vertically, LayoutBox, SizeConstraint, VerticalLayoutStrategy};
    use futures_signals::signal::{always, Mutable, SignalExt};
    use futures_signals::signal_vec::MutableVec;
    use glam::{UVec2, uvec2};
    use crate::widgets::{List, Slab};

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
    fn list_test() {
        let list = List {
            children: MutableVec::new_with_values(vec![
                Box::new(Slab {})
            ]),
            requested_size: Mutable::new(SizeConstraint::MinSize(uvec2(10, 100))),
        };
    }
}

#[macro_export]
macro_rules! assert_f32_eq {
    ($l:expr, $r:expr, $msg:expr) => {{
        let diff = ($l-$r).abs();

        if diff > f32::EPSILON {
            panic!("{} - f32 difference {} is greater than allowed diff f32::EPSILON", $msg, diff);
        }
    }}
}