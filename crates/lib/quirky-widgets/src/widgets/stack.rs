use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use futures_signals::signal::SignalExt;
use quirky::quirky_app_context::QuirkyAppContext;
use quirky::run_widgets;
use quirky::widget::{Widget, WidgetBase};
use quirky_macros::widget;
use std::sync::Arc;

/// A stack of layered components sharing the same bounding box
#[widget]
pub struct Stack {
    #[signal_prop]
    #[default(vec![])]
    children: Vec<Arc<dyn Widget>>,
}

#[async_trait]
impl<
        ChildrenSignal: futures_signals::signal::Signal<Item = Vec<Arc<dyn Widget>>>
            + Send
            + Sync
            + Unpin
            + 'static,
        ChildrenSignalFn: Fn() -> ChildrenSignal + Send + Sync + 'static,
    > Widget for Stack<ChildrenSignal, ChildrenSignalFn>
{
    fn children(&self) -> Option<Vec<Arc<dyn Widget>>> {
        let children = self.children_prop_value.get_cloned();
        children
    }

    async fn run(self: Arc<Self>, ctx: &QuirkyAppContext) {
        let mut futs = self.poll_prop_futures(ctx);

        let widgets_run = run_widgets(
            ctx,
            self.children_prop_value
                .signal_cloned()
                .map(|v| v.unwrap())
                .to_signal_vec(),
        );

        let bb_update = self.bounding_box.signal().for_each(|new_bb| {
            self.children_prop_value
                .lock_ref()
                .as_ref()
                .unwrap()
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
