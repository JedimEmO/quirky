use futures::stream::FuturesUnordered;
use futures::FutureExt;
use futures::{select, StreamExt};
use futures_signals::signal::{Signal, SignalExt};
use futures_signals::signal_vec::{MutableVec, SignalVecExt};
use quirky::quirky_app_context::QuirkyAppContext;
use quirky::widget::Widget;
use quirky::{layout, LayoutBox, SizeConstraint};
use std::sync::Arc;

pub async fn run_widget_with_children<'a, TExtras: Send>(
    widget: Arc<dyn Widget>,
    children_data: MutableVec<Arc<dyn Widget>>,
    ctx: &QuirkyAppContext,
    widget_children: impl Signal<Item = Vec<Arc<dyn Widget>>> + Unpin,
    extras_signal: impl Signal<Item = TExtras> + Send,
    layout_strategy: impl Fn(&LayoutBox, &Vec<SizeConstraint>, &TExtras) -> Vec<LayoutBox> + Send,
) {
    let children = children_data.signal_vec_cloned().to_signal_cloned();

    let child_layouts = layout(
        widget.bounding_box().signal(),
        children.map(|v| v.into_iter().map(|c| c.size_constraint()).collect()),
        extras_signal,
        layout_strategy,
    );

    let mut child_layouts_stream = child_layouts.to_stream();
    let mut child_run_futs = FuturesUnordered::new();
    let mut children_stream = widget_children.to_stream().fuse();

    loop {
        let mut next_layouts = child_layouts_stream.next().fuse();
        let mut next_child_run_fut = child_run_futs.next();
        let mut next_children = children_stream.select_next_some();

        select! {
            layouts = next_layouts => {
                if let Some(layouts) = layouts {
                    let _layout_lock = ctx.start_layout();
                    child_run_futs = FuturesUnordered::new();

                    layouts.iter().enumerate().for_each(|(idx, l)| {
                        let child = children_data.lock_ref()[idx].clone();

                        child.set_bounding_box(*l);
                        child_run_futs.push(child.run(ctx));
                    });
                }
            }

            _childruns = next_child_run_fut => {}

            new_children = next_children => {
                children_data.lock_mut().replace_cloned(new_children);
            }
        }
    }
}
