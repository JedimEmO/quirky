use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use futures_signals::signal::{Signal, SignalExt};
use futures_signals::signal_vec::SignalVecExt;
use quirky::quirky_app_context::QuirkyAppContext;
use quirky::widget::{Widget, WidgetBase};
use quirky::{run_widgets, SizeConstraint};
use quirky_macros::widget;
use std::sync::Arc;

/// A stack of layered components sharing the same bounding box
#[widget]
pub struct Stack {
    #[signal_vec_prop]
    #[default(vec![])]
    children: Arc<dyn Widget>,

    #[signal_prop]
    #[default(Default::default())]
    size_constraint: SizeConstraint,
}

#[async_trait]
impl<
        SizeConstraintSignal: futures_signals::signal::Signal<Item = SizeConstraint> + Send + Sync + Unpin + 'static,
        SizeConstraintSignalFn: Fn() -> SizeConstraintSignal + Send + Sync + 'static,
        ChildrenSignal: futures_signals::signal_vec::SignalVec<Item = Arc<dyn Widget>>
            + Send
            + Sync
            + Unpin
            + 'static,
        ChildrenSignalFn: Fn() -> ChildrenSignal + Send + Sync + 'static,
    > Widget
    for Stack<SizeConstraintSignal, SizeConstraintSignalFn, ChildrenSignal, ChildrenSignalFn>
{
    fn children(&self) -> Option<Vec<Arc<dyn Widget>>> {
        let children = self.children_prop_value.lock_ref().to_vec();
        Some(children)
    }

    fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send> {
        Box::new(
            self.size_constraint_prop_value
                .signal()
                .map(|v| v.or(Some(SizeConstraint::Unconstrained)).unwrap())
                .dedupe(),
        )
    }

    async fn run(self: Arc<Self>, ctx: &QuirkyAppContext) {
        let mut futs = self.poll_prop_futures(ctx);

        let widgets_run = run_widgets(ctx, self.children_prop_value.signal_vec_cloned());

        let bb_update = self.bounding_box.signal().for_each(|new_bb| {
            self.children_prop_value
                .lock_ref()
                .iter()
                .for_each(|child| {
                    child.set_bounding_box(new_bb);
                });

            async move {
                ctx.signal_redraw().await;
            }
        });

        futs.push(widgets_run.boxed());
        futs.push(bb_update.boxed());

        loop {
            let _ = futs.select_next_some().await;
        }
    }
}
