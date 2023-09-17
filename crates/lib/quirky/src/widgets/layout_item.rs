use crate::quirky_app_context::QuirkyAppContext;
use crate::styling::Padding;
use crate::widget::Widget;
use crate::widget::WidgetBase;
use crate::SizeConstraint;
use crate::{layout, LayoutBox};
use async_trait::async_trait;
use futures::stream::FuturesUnordered;
use futures::FutureExt;
use futures::{select, StreamExt};
use futures_signals::signal::{always, Mutable, Signal, SignalExt};
use glam::UVec2;
use quirky_macros::widget;
use std::sync::Arc;
use uuid::Uuid;

#[widget]
pub struct LayoutItem {
    #[signal_prop]
    child: Arc<dyn Widget>,
    #[signal_prop]
    #[default(Default::default())]
    padding: Padding,
}

#[async_trait]
impl<
        ChildSignal: futures_signals::signal::Signal<Item = Arc<dyn Widget>> + Send + Sync + Unpin + 'static,
        ChildSignalFn: Fn() -> ChildSignal + Send + Sync + 'static,
        PaddingSignal: futures_signals::signal::Signal<Item = Padding> + Send + Sync + Unpin + 'static,
        PaddingSignalFn: Fn() -> PaddingSignal + Send + Sync + 'static,
    > Widget for LayoutItem<ChildSignal, ChildSignalFn, PaddingSignal, PaddingSignalFn>
{
    fn children(&self) -> Option<Vec<Arc<dyn Widget>>> {
        self.child_prop_value.get_cloned().map(|v| vec![v])
    }

    fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send> {
        Box::new(always(SizeConstraint::Unconstrained))
    }

    fn get_widget_at(&self, pos: UVec2, path: Vec<Uuid>) -> Option<Vec<Uuid>> {
        self.child_prop_value
            .get_cloned()
            .map(|c| c.get_widget_at(pos, path))
            .flatten()
    }

    async fn run(self: Arc<Self>, ctx: &QuirkyAppContext) {
        let children = self.child_prop_value.signal_cloned();

        let child_layouts = layout(
            self.bounding_box().signal(),
            children.map(|v| v.into_iter().map(|c| c.size_constraint()).collect()),
            (self.padding)(),
            layout_item_strategy,
        );

        let mut child_layouts_stream = child_layouts.to_stream();
        let mut child_run_futs = self.poll_prop_futures(ctx);

        loop {
            let mut next_layouts = child_layouts_stream.next().fuse();
            let mut next_child_run_fut = child_run_futs.next();

            select! {
                layouts = next_layouts => {
                    if let Some(layouts) = layouts {
                        let _layout_lock = ctx.start_layout();
                        child_run_futs = FuturesUnordered::new();

                        layouts.iter().enumerate().for_each(|(_idx, l)| {
                            if let Some(child) = self.child_prop_value.get_cloned() {
                                child.set_bounding_box(*l);
                                child_run_futs.push(child.run(ctx));
                            }
                        });
                    }
                }

                _childruns = next_child_run_fut => {}
            }
        }
    }
}

fn layout_item_strategy(
    container_box: &LayoutBox,
    child_constraints: &Vec<SizeConstraint>,
    padding: &Padding,
) -> Vec<LayoutBox> {
    if child_constraints.len() == 0 {
        return vec![];
    }

    let padding_total = UVec2::new(padding.left + padding.right, padding.top + padding.bottom);

    if container_box.size.x < padding_total.x || container_box.size.y < padding_total.y {
        return vec![];
    }

    let top_left = container_box.pos + UVec2::new(padding.left, padding.top);
    let size = container_box.size - padding_total;

    vec![LayoutBox {
        pos: top_left,
        size,
    }]
}
