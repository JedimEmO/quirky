use crate::drawables::Drawable;

use crate::widget::Widget;
use crate::{layout, LayoutBox, SizeConstraint};
use async_trait::async_trait;
use futures::stream::FuturesUnordered;
use futures::FutureExt;
use futures::{select, StreamExt};
use futures_signals::signal::{Mutable, ReadOnlyMutable, Signal, SignalExt};
use futures_signals::signal_vec::{MutableVec, SignalVecExt};
use glam::UVec2;
use std::sync::Arc;
use typed_builder::TypedBuilder;
use wgpu::Device;

#[derive(Copy, Clone)]
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
        TChildrenFn: Fn() -> TChildrenSignal + Send + Sync,
        TChildrenSignal: Signal<Item = Vec<Arc<dyn Widget>>> + Send + Unpin,
        TChildDirectionFn: Fn() -> TChildDirectionSignal + Send + Sync,
        TChildDirectionSignal: Signal<Item = ChildDirection>,
        TSizeConstraintsFn: Fn() -> TSizeConstraintsSignal + Send + Sync,
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

    async fn run(self: Arc<Self>, drawable_data: MutableVec<Drawable>, device: &Device) {
        let children_data: MutableVec<Arc<dyn Widget>> = MutableVec::new();
        let children = children_data.signal_vec_cloned().to_signal_cloned();

        let child_layouts = layout(
            self.bounding_box().signal(),
            children.map(|v| v.into_iter().map(|c| c.size_constraint()).collect()),
            |container_box, child_constraints| {
                let total_items = child_constraints.len().max(1) as u32;

                let min_requirements_x: u32 = child_constraints
                    .iter()
                    .map(|r| match r {
                        SizeConstraint::MinSize(s) => s.x,
                        _ => 0,
                    })
                    .sum();

                let remaining_width = if container_box.size.x < min_requirements_x {
                    container_box.size.x
                } else {
                    container_box.size.x - min_requirements_x
                };
                let per_remaining_width_bonus = remaining_width / total_items;

                let mut x_pos = 0;

                child_constraints
                    .iter()
                    .map(|i| {
                        let base_x = match i {
                            SizeConstraint::MinSize(s) => s.x,
                            _ => 0,
                        };

                        let item_width = base_x + per_remaining_width_bonus;
                        let pos = x_pos;
                        x_pos += item_width;

                        LayoutBox {
                            pos: UVec2::new(pos, 0),
                            size: UVec2::new(item_width, container_box.size.y),
                        }
                    })
                    .collect()
            },
        );

        let mut child_layouts_stream = child_layouts.to_stream();
        let mut child_run_futs = FuturesUnordered::new();
        let mut children_stream = (self.children)().to_stream().fuse();

        loop {
            let mut next_layouts = child_layouts_stream.next().fuse();
            let mut next_child_run_fut = child_run_futs.next();
            let mut next_children = children_stream.select_next_some();

            select! {
                layouts = next_layouts => {
                    if let Some(layouts) = layouts {
                        child_run_futs = FuturesUnordered::new();

                        let mut new_drawables = self.paint(device);

                        layouts.iter().enumerate().for_each(|(idx, l)| {
                            let child = children_data.lock_ref()[idx].clone();

                            child.set_bounding_box(*l);
                            let child_subtree = MutableVec::new_with_values(child.paint(device));
                            new_drawables.push(Drawable::SubTree {children: child_subtree.clone(), transform: l.pos, size: l.size });
                            child_run_futs.push(child.run(child_subtree, device));
                        });

                        drawable_data.lock_mut().replace_cloned(new_drawables);
                    }
                }

                _childruns = next_child_run_fut => {}

                new_children = next_children => {
                    children_data.lock_mut().replace_cloned(new_children);
                }
            }
        }
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
