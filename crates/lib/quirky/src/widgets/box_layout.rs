use crate::drawables::Drawable;

use crate::widget::Widget;
use crate::widgets::run_widget_with_children::run_widget_with_children;
use crate::{LayoutBox, QuirkyAppContext, SizeConstraint};
use async_trait::async_trait;
use futures_signals::signal::{Mutable, ReadOnlyMutable, Signal};
use futures_signals::signal_vec::MutableVec;
use glam::UVec2;
use std::sync::Arc;
use typed_builder::TypedBuilder;
use wgpu::Device;

#[derive(Copy, Clone, PartialEq)]
pub enum ChildDirection {
    Horizontal,
    Vertical,
}

#[derive(TypedBuilder)]
pub struct BoxLayout<
    TChildrenFn: Fn() -> TChildrenSignal,
    TChildrenSignal: Signal<Item = Vec<Arc<dyn Widget>>>,
    TChildDirectionFn: Fn() -> TChildDirectionSignal,
    TChildDirectionSignal: Signal<Item = ChildDirection>,
    TSizeConstraintsFn: Fn() -> TSizeConstraintsSignal,
    TSizeConstraintsSignal: Signal<Item = SizeConstraint>,
> {
    pub children: TChildrenFn,
    pub child_direction: TChildDirectionFn,
    pub size_constraint: TSizeConstraintsFn,
    #[builder(default)]
    pub bounding_box: Mutable<LayoutBox>,
}

#[async_trait]
impl<
        TChildrenFn: Fn() -> TChildrenSignal + Send + Sync + 'static,
        TChildrenSignal: Signal<Item = Vec<Arc<dyn Widget>>> + Send + Unpin + 'static,
        TChildDirectionFn: Fn() -> TChildDirectionSignal + Send + Sync + 'static,
        TChildDirectionSignal: Signal<Item = ChildDirection> + Send + 'static,
        TSizeConstraintsFn: Fn() -> TSizeConstraintsSignal + Send + Sync + 'static,
        TSizeConstraintsSignal: Signal<Item = SizeConstraint> + Unpin + Send + 'static,
    > Widget
    for BoxLayout<
        TChildrenFn,
        TChildrenSignal,
        TChildDirectionFn,
        TChildDirectionSignal,
        TSizeConstraintsFn,
        TSizeConstraintsSignal,
    >
{
    fn paint(&self, _device: &Device) -> Vec<Drawable> {
        let _bb = self.bounding_box.get();

        vec![]
    }

    fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send> {
        Box::new((self.size_constraint)())
    }

    fn set_bounding_box(&self, new_box: LayoutBox) {
        self.bounding_box.set(new_box);
    }

    fn bounding_box(&self) -> ReadOnlyMutable<LayoutBox> {
        self.bounding_box.read_only()
    }

    async fn run(
        self: Arc<Self>,
        ctx: &QuirkyAppContext,
        drawable_data: MutableVec<Drawable>,
        device: &Device,
    ) {
        run_widget_with_children(
            self.clone(),
            ctx,
            drawable_data,
            (self.children)(),
            (self.child_direction)(),
            box_layout_strategy,
            device,
        )
        .await;
    }
}

fn box_layout_strategy(
    container_box: &LayoutBox,
    child_constraints: &Vec<SizeConstraint>,
    direction: &ChildDirection,
) -> Vec<LayoutBox> {
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

    if direction == &ChildDirection::Horizontal {
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
    use crate::widgets::box_layout::BoxLayout;
    use crate::{clone, SizeConstraint};
    use futures_signals::signal::{always, Mutable};

    #[test]
    fn box_layout_usage() {
        let constraint = Mutable::new(SizeConstraint::Unconstrained);

        let _box_layout_props = BoxLayout::builder()
            .children(|| always(vec![]))
            .child_direction(|| always(super::ChildDirection::Vertical))
            .size_constraint(clone!(constraint, move || constraint.signal()))
            .bounding_box(Default::default())
            .build();
    }
}
