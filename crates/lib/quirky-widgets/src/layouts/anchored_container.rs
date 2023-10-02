use crate::styling::Padding;
use async_trait::async_trait;
use futures::FutureExt;
use futures::StreamExt;
use futures_signals::map_ref;
use futures_signals::signal::{always, Signal, SignalExt};
use glam::UVec2;
use quirky::quirky_app_context::QuirkyAppContext;
use quirky::widget::WidgetBase;
use quirky::widget::{SizeConstraint, Widget};
use quirky::widgets::layout_helper::layout;
use quirky::LayoutBox;
use quirky_macros::widget;
use quirky_utils::futures_map_poll::FuturesMapPoll;
use std::cmp::min;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum AnchorPoint {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}
#[widget]
pub struct AnchoredContainer {
    #[signal_prop]
    child: Arc<dyn Widget>,
    #[signal_prop]
    #[default(AnchorPoint::Center)]
    anchor_point: AnchorPoint,
    #[signal_prop]
    #[default(Default::default())]
    padding: Padding,
}

#[async_trait]
impl<
        ChildSignal: futures_signals::signal::Signal<Item = Arc<dyn Widget>> + Send + Sync + Unpin + 'static,
        ChildSignalFn: Fn() -> ChildSignal + Send + Sync + 'static,
        AnchorPointSignal: futures_signals::signal::Signal<Item = AnchorPoint> + Send + Sync + Unpin + 'static,
        AnchorPointSignalFn: Fn() -> AnchorPointSignal + Send + Sync + 'static,
        PaddingSignal: futures_signals::signal::Signal<Item = Padding> + Send + Sync + Unpin + 'static,
        PaddingSignalFn: Fn() -> PaddingSignal + Send + Sync + 'static,
    > Widget
    for AnchoredContainer<
        ChildSignal,
        ChildSignalFn,
        AnchorPointSignal,
        AnchorPointSignalFn,
        PaddingSignal,
        PaddingSignalFn,
    >
{
    fn children(&self) -> Option<Vec<Arc<dyn Widget>>> {
        self.child_prop_value.get_cloned().map(|v| vec![v])
    }

    fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send> {
        Box::new(
            self.child_prop_value
                .signal_cloned()
                .map(|child| {
                    child
                        .map(|child| {
                            child
                                .size_constraint()
                                .map(|child_constraint| match child_constraint {
                                    SizeConstraint::MinSize(size) => SizeConstraint::MinSize(size),
                                    _ => SizeConstraint::Unconstrained,
                                })
                                .boxed()
                        })
                        .or(Some(always(SizeConstraint::Unconstrained).boxed()))
                        .unwrap()
                })
                .flatten(),
        )
    }

    fn get_widget_at(&self, pos: UVec2, path: Vec<Uuid>) -> Option<Vec<Uuid>> {
        self.child_prop_value
            .get_cloned()
            .map(|c| c.get_widget_at(pos, path))
            .flatten()
    }

    async fn run(self: Arc<Self>, ctx: &QuirkyAppContext) {
        let mut futs = self.poll_prop_futures(ctx);

        let children = self.child_prop_value.signal_cloned();

        let extras = map_ref! {
            let padding = self.padding_prop_value.signal().map(|v| v.unwrap()),
            let anchor_point = self.anchor_point_prop_value.signal().map(|v| v.unwrap()) => {
                (*padding, *anchor_point)
            }
        };

        let child_constraints = children
            .map(|v| {
                v.map(|c| vec![c.size_constraint()])
                    .or(Some(vec![]))
                    .unwrap()
            })
            .to_signal_vec();

        let child_layouts = layout(
            self.bounding_box().signal(),
            child_constraints,
            extras,
            single_child_layout_strategy,
        )
        .for_each(|layouts| {
            layouts.iter().enumerate().for_each(|(_idx, l)| {
                self.child_prop_value
                    .lock_ref()
                    .as_ref()
                    .map(|v| v.set_bounding_box(*l));
            });

            async move {
                ctx.signal_redraw().await;
            }
        });

        let (pollable, data) = FuturesMapPoll::new();

        let child_runner = self
            .child_prop_value
            .signal_cloned()
            .for_each(move |child| {
                if let Some(child) = child {
                    data.clear();
                    data.insert(&child.id(), child.run(ctx));
                }
                async move {}
            });

        futs.push(pollable.boxed());
        futs.push(child_runner.boxed());
        futs.push(child_layouts.boxed());

        loop {
            let _ = futs.select_next_some().await;
        }
    }
}

pub fn single_child_layout_strategy(
    container_box: &LayoutBox,
    child_constraints: &Vec<SizeConstraint>,
    extras: &(Padding, AnchorPoint),
) -> Vec<LayoutBox> {
    if child_constraints.len() == 0 {
        return vec![];
    }

    let padding = extras.0;
    let anchor_point = extras.1;

    let child_constraints = child_constraints.first().unwrap();

    let padding_total = UVec2::new(padding.left + padding.right, padding.top + padding.bottom);

    if container_box.size.x < padding_total.x || container_box.size.y < padding_total.y {
        return vec![];
    }

    let top_left = container_box.pos + UVec2::new(padding.left, padding.top);
    let size = container_box.size - padding_total;

    let allowed_box = LayoutBox {
        pos: top_left,
        size,
    };

    let allocated_size = match child_constraints {
        SizeConstraint::Unconstrained => allowed_box.size,
        SizeConstraint::MaxSize(max_size) => {
            let size_x = min(allowed_box.size.x, max_size.x);
            let size_y = min(allowed_box.size.y, max_size.y);

            UVec2::new(size_x, size_y)
        }
        SizeConstraint::MinSize(_) => {
            // For single child layouts, MinSize equates to unconstrained as it only makes sense to
            // consume the allowed space
            allowed_box.size
        }
        SizeConstraint::MaxHeight(max_height) => {
            let size_x = allowed_box.size.x;
            let size_y = min(allowed_box.size.y, *max_height);

            UVec2::new(size_x, size_y)
        }
        SizeConstraint::MaxWidth(max_width) => {
            let size_x = min(allowed_box.size.x, *max_width);
            let size_y = allowed_box.size.y;

            UVec2::new(size_x, size_y)
        }
    };

    let output_pos = match anchor_point {
        AnchorPoint::Center => {
            let allowed_center_x = allowed_box.size.x / 2;
            let allowed_center_y = allowed_box.size.y / 2;

            let allocated_half_x = allocated_size.x / 2;
            let allocated_half_y = allocated_size.y / 2;

            let pos_x = if allocated_half_x <= allowed_center_x {
                allowed_center_x - allocated_half_x
            } else {
                0
            };

            let pos_y = if allocated_half_y <= allowed_center_y {
                allowed_center_y - allocated_half_y
            } else {
                0
            };

            top_left + UVec2::new(pos_x, pos_y)
        }
        AnchorPoint::TopLeft => top_left,
        AnchorPoint::TopCenter => {
            let allowed_center_x = allowed_box.size.x / 2;
            let allocated_half_x = allocated_size.x / 2;

            let pos_x = if allocated_half_x <= allowed_center_x {
                allowed_center_x - allocated_half_x
            } else {
                0
            };

            top_left + UVec2::new(pos_x, 0)
        }
        AnchorPoint::TopRight => {
            let pos_x = allowed_box.size.x - allocated_size.x;

            top_left + UVec2::new(pos_x, 0)
        }
        AnchorPoint::CenterLeft => {
            let allowed_center_y = allowed_box.size.y / 2;
            let allocated_half_y = allocated_size.y / 2;

            let pos_y = if allocated_half_y <= allowed_center_y {
                allowed_center_y - allocated_half_y
            } else {
                0
            };

            top_left + UVec2::new(0, pos_y)
        }
        AnchorPoint::CenterRight => {
            let allowed_center_x = allowed_box.size.x;
            let allocated_half_x = allocated_size.x;

            let pos_x = if allocated_half_x <= allowed_center_x {
                allowed_center_x - allocated_half_x
            } else {
                0
            };

            let allowed_center_y = allowed_box.size.y / 2;
            let allocated_half_y = allocated_size.y / 2;

            let pos_y = if allocated_half_y <= allowed_center_y {
                allowed_center_y - allocated_half_y
            } else {
                0
            };

            top_left + UVec2::new(pos_x, pos_y)
        }
        AnchorPoint::BottomLeft => {
            let pos_y = allowed_box.size.y - allocated_size.y;

            top_left + UVec2::new(0, pos_y)
        }
        AnchorPoint::BottomCenter => {
            let allowed_center_x = allowed_box.size.x / 2;
            let allocated_half_x = allocated_size.x / 2;

            let pos_x = if allocated_half_x <= allowed_center_x {
                allowed_center_x - allocated_half_x
            } else {
                0
            };

            let pos_y = allowed_box.size.y - allocated_size.y;

            top_left + UVec2::new(pos_x, pos_y)
        }
        AnchorPoint::BottomRight => {
            let pos_x = allowed_box.size.x - allocated_size.x;
            let pos_y = allowed_box.size.y - allocated_size.y;

            top_left + UVec2::new(pos_x, pos_y)
        }
    };

    vec![LayoutBox {
        pos: output_pos,
        size: allocated_size,
    }]
}
