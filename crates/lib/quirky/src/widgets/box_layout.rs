use crate::primitives::{DrawablePrimitive, PrepareContext};
use crate::quirky_app_context::QuirkyAppContext;
use crate::widget::{Widget, WidgetBase};
use crate::widgets::run_widget_with_children::run_widget_with_children;
use crate::{LayoutBox, SizeConstraint};
use async_trait::async_trait;
use futures_signals::signal::Signal;
use futures_signals::signal_vec::MutableVec;
use glam::UVec2;
use quirky_macros::widget;
use std::sync::Arc;
use uuid::Uuid;
use wgpu::Device;

#[derive(Copy, Clone, PartialEq)]
pub enum ChildDirection {
    Horizontal,
    Vertical,
}

#[widget]
pub struct BoxLayout {
    #[signal_prop]
    pub children: Vec<Arc<dyn Widget>>,
    #[signal_prop]
    #[default(ChildDirection::Vertical)]
    pub child_direction: ChildDirection,
    #[signal_prop]
    #[default(SizeConstraint::Unconstrained)]
    pub size_constraint: SizeConstraint,
    child_data: MutableVec<Arc<dyn Widget>>,
}

#[async_trait]
impl<
        ChildrenSignal: futures_signals::signal::Signal<Item = Vec<Arc<dyn Widget>>>
            + Send
            + Sync
            + Unpin
            + 'static,
        ChildrenSignalFn: Fn() -> ChildrenSignal + Send + Sync + 'static,
        ChildDirectionSignal: futures_signals::signal::Signal<Item = ChildDirection> + Send + Sync + Unpin + 'static,
        ChildDirectionSignalFn: Fn() -> ChildDirectionSignal + Send + Sync + 'static,
        SizeConstraintSignal: futures_signals::signal::Signal<Item = SizeConstraint> + Send + Sync + Unpin + 'static,
        SizeConstraintSignalFn: Fn() -> SizeConstraintSignal + Send + Sync + 'static,
    > Widget
    for BoxLayout<
        ChildrenSignal,
        ChildrenSignalFn,
        ChildDirectionSignal,
        ChildDirectionSignalFn,
        SizeConstraintSignal,
        SizeConstraintSignalFn,
    >
{
    fn children(&self) -> Option<Vec<Arc<dyn Widget>>> {
        Some(self.child_data.lock_ref().to_vec())
    }

    fn paint(
        &self,
        _quirky_context: &QuirkyAppContext,
        paint_ctx: &mut PrepareContext,
    ) -> Vec<Box<dyn DrawablePrimitive>> {
        let _bb = self.bounding_box.get();

        vec![]
    }

    fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send> {
        Box::new((self.size_constraint)())
    }

    fn get_widget_at(&self, pos: UVec2, path: Vec<Uuid>) -> Option<Vec<Uuid>> {
        let bb = self.bounding_box.get();

        if !bb.contains(pos) {
            return None;
        }

        let child_data = self.child_data.lock_ref();

        for c in child_data.iter().rev() {
            if let Some(mut out_path) = c.get_widget_at(pos, path.clone()) {
                out_path.push(self.id);
                return Some(out_path);
            }
        }

        None
    }

    async fn run(self: Arc<Self>, ctx: &QuirkyAppContext, device: &Device) {
        run_widget_with_children(
            self.clone(),
            self.child_data.clone(),
            ctx,
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
    use crate::widgets::box_layout::BoxLayoutBuilder;
    use crate::{clone, SizeConstraint};
    use futures_signals::signal::Mutable;

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
