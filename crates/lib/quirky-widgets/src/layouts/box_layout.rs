use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use futures_signals::map_ref;
use futures_signals::signal::Signal;
use futures_signals::signal::SignalExt;
use futures_signals::signal_vec::SignalVecExt;
use futures_signals::signal_vec::VecDiff;
use glam::UVec2;
use quirky::quirky_app_context::QuirkyAppContext;
use quirky::styling::Padding;
use quirky::widget::{Widget, WidgetBase};
use quirky::{layout, LayoutBox, SizeConstraint};
use quirky_macros::widget;
use quirky_utils::futures_map_poll::FuturesMapPoll;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Copy, Clone, PartialEq)]
pub enum ChildDirection {
    Horizontal,
    Vertical,
}

#[widget]
pub struct BoxLayout {
    #[signal_vec_prop]
    pub children: Arc<dyn Widget>,
    #[signal_prop]
    #[default(ChildDirection::Vertical)]
    pub child_direction: ChildDirection,
    #[signal_prop]
    #[default(SizeConstraint::Unconstrained)]
    pub size_constraint: SizeConstraint,
    #[signal_prop]
    #[default(Default::default())]
    padding: Padding,
}

#[async_trait]
impl<
        ChildDirectionSignal: futures_signals::signal::Signal<Item = ChildDirection> + Send + Sync + Unpin + 'static,
        ChildDirectionSignalFn: Fn() -> ChildDirectionSignal + Send + Sync + 'static,
        SizeConstraintSignal: futures_signals::signal::Signal<Item = SizeConstraint> + Send + Sync + Unpin + 'static,
        SizeConstraintSignalFn: Fn() -> SizeConstraintSignal + Send + Sync + 'static,
        PaddingSignal: futures_signals::signal::Signal<Item = Padding> + Send + Sync + Unpin + 'static,
        PaddingSignalFn: Fn() -> PaddingSignal + Send + Sync + 'static,
        ChildrenSignal: futures_signals::signal_vec::SignalVec<Item = Arc<dyn Widget>>
            + Send
            + Sync
            + Unpin
            + 'static,
        ChildrenSignalFn: Fn() -> ChildrenSignal + Send + Sync + 'static,
    > Widget
    for BoxLayout<
        ChildDirectionSignal,
        ChildDirectionSignalFn,
        SizeConstraintSignal,
        SizeConstraintSignalFn,
        PaddingSignal,
        PaddingSignalFn,
        ChildrenSignal,
        ChildrenSignalFn,
    >
{
    fn children(&self) -> Option<Vec<Arc<dyn Widget>>> {
        Some(self.children_prop_value.lock_ref().to_vec())
    }

    fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send> {
        Box::new((self.size_constraint)())
    }

    fn get_widget_at(&self, pos: UVec2, path: Vec<Uuid>) -> Option<Vec<Uuid>> {
        let bb = self.bounding_box.get();

        if !bb.contains(pos) {
            return None;
        }

        let child_data = self.children_prop_value.lock_ref();

        for c in child_data.iter().rev() {
            if let Some(mut out_path) = c.get_widget_at(pos, path.clone()) {
                out_path.push(self.id);
                return Some(out_path);
            }
        }

        None
    }

    async fn run(self: Arc<Self>, ctx: &QuirkyAppContext) {
        let mut futs = self.poll_prop_futures(ctx);

        let child_constraints_signal = self
            .children_prop_value
            .signal_vec_cloned()
            .map(|c| c.size_constraint());

        let child_futs = layout(
            self.bounding_box().signal(),
            child_constraints_signal,
            self.layout_extras(),
            box_layout_strategy,
        )
        .for_each(|layouts| {
            let children = self.children_prop_value.lock_mut();

            layouts.iter().enumerate().for_each(|(idx, l)| {
                children[idx].set_bounding_box(*l);
            });

            async {}
        });

        let (children_pollable_map, children_data_map) = FuturesMapPoll::new();

        let child_run_fut =
            self.children_prop_value
                .signal_vec_cloned()
                .for_each(move |children_diff| {
                    match children_diff {
                        VecDiff::Clear {} => children_data_map.clear(),
                        VecDiff::InsertAt { value, .. } => {
                            children_data_map.insert(&value.id(), value.run(ctx));
                        }
                        VecDiff::Push { value } => {
                            children_data_map.insert(&value.id(), value.run(ctx));
                        }
                        VecDiff::Replace { values } => {
                            children_data_map.clear();
                            values.into_iter().for_each(|v| {
                                children_data_map.insert(&v.id(), v.run(ctx));
                            });
                        }
                        VecDiff::UpdateAt { value, .. } => {
                            children_data_map.insert(&value.id(), value.run(ctx));
                        }
                        _ => {}
                    }
                    async {}
                });

        futs.push(child_futs.boxed());
        futs.push(child_run_fut.boxed());
        futs.push(children_pollable_map.boxed());

        loop {
            let _ = futs.select_next_some().await;
        }
    }
}

impl<
        ChildDirectionSignal: futures_signals::signal::Signal<Item = ChildDirection> + Send + Sync + Unpin + 'static,
        ChildDirectionSignalFn: Fn() -> ChildDirectionSignal + Send + Sync + 'static,
        SizeConstraintSignal: futures_signals::signal::Signal<Item = SizeConstraint> + Send + Sync + Unpin + 'static,
        SizeConstraintSignalFn: Fn() -> SizeConstraintSignal + Send + Sync + 'static,
        PaddingSignal: futures_signals::signal::Signal<Item = Padding> + Send + Sync + Unpin + 'static,
        PaddingSignalFn: Fn() -> PaddingSignal + Send + Sync + 'static,
        ChildrenSignal: futures_signals::signal_vec::SignalVec<Item = Arc<dyn Widget>>
            + Send
            + Sync
            + Unpin
            + 'static,
        ChildrenSignalFn: Fn() -> ChildrenSignal + Send + Sync + 'static,
    >
    BoxLayout<
        ChildDirectionSignal,
        ChildDirectionSignalFn,
        SizeConstraintSignal,
        SizeConstraintSignalFn,
        PaddingSignal,
        PaddingSignalFn,
        ChildrenSignal,
        ChildrenSignalFn,
    >
{
    fn layout_extras(&self) -> impl Signal<Item = (ChildDirection, Padding)> {
        map_ref! {
            let padding = (self.padding)(),
            let direction = (self.child_direction)() => {
                (*direction, *padding)
            }
        }
    }
}

fn box_layout_strategy(
    container_box: &LayoutBox,
    child_constraints: &Vec<SizeConstraint>,
    extras: &(ChildDirection, Padding),
) -> Vec<LayoutBox> {
    let direction = extras.0;
    let padding = extras.1;

    let padding_total = UVec2::new(padding.left + padding.right, padding.top + padding.bottom);

    if container_box.size.x < padding_total.x || container_box.size.y < padding_total.y {
        return vec![];
    }

    let top_left = container_box.pos + UVec2::new(padding.left, padding.top);
    let size = container_box.size - padding_total;

    let container_box = LayoutBox {
        pos: top_left,
        size,
    };

    let total_items = child_constraints.len().max(1) as u32;

    let min_requirements_x: u32 = child_constraints
        .iter()
        .map(|r| match r {
            SizeConstraint::MinSize(s) => s.x,
            _ => 0,
        })
        .sum();

    let min_requirements_y: u32 = child_constraints
        .iter()
        .map(|r| match r {
            SizeConstraint::MinSize(s) => s.y,
            _ => 0,
        })
        .sum();

    if direction == ChildDirection::Horizontal {
        let remaining_width = if container_box.size.x < min_requirements_x {
            container_box.size.x
        } else {
            container_box.size.x - min_requirements_x
        };

        let mut per_remaining_width_bonus = remaining_width / total_items;
        let mut x_pos = 0;

        child_constraints
            .iter()
            .map(|i| {
                let base_x = match i {
                    SizeConstraint::MinSize(s) => s.x,
                    _ => 0,
                };

                let item_width = base_x + per_remaining_width_bonus;
                let item_width = if let SizeConstraint::MaxWidth(wm) = i {
                    if item_width > *wm {
                        per_remaining_width_bonus += item_width - wm;
                    }

                    *wm.min(&item_width)
                } else {
                    item_width
                };

                let pos = x_pos;
                x_pos += item_width;

                LayoutBox {
                    pos: UVec2::new(container_box.pos.x + pos, container_box.pos.y),
                    size: UVec2::new(item_width, container_box.size.y),
                }
            })
            .collect()
    } else {
        let remaining_height = if container_box.size.y < min_requirements_y {
            container_box.size.y
        } else {
            container_box.size.y - min_requirements_y
        };

        let mut per_remaining_height_bonus = remaining_height / total_items;

        let mut y_pos = 0;

        child_constraints
            .iter()
            .map(|i| {
                let base_y = match i {
                    SizeConstraint::MinSize(s) => s.y,
                    _ => 0,
                };

                let item_height = base_y + per_remaining_height_bonus;
                let item_height = if let SizeConstraint::MaxHeight(hm) = i {
                    if item_height > *hm {
                        per_remaining_height_bonus += item_height - hm;
                    }

                    *hm.min(&item_height)
                } else {
                    item_height
                };

                let pos = y_pos;
                y_pos += item_height;

                LayoutBox {
                    pos: UVec2::new(container_box.pos.x, container_box.pos.y + pos),
                    size: UVec2::new(container_box.size.x, item_height),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use crate::layouts::box_layout::BoxLayoutBuilder;
    use futures_signals::signal::Mutable;
    use quirky::{clone, SizeConstraint};

    #[test]
    fn box_layout_usage() {
        let constraint = Mutable::new(SizeConstraint::Unconstrained);

        let _box_layout_props = BoxLayoutBuilder::new()
            .children(vec![])
            .child_direction(super::ChildDirection::Vertical)
            .size_constraint_signal(clone!(constraint, move || constraint.signal()))
            .build();
    }
}
