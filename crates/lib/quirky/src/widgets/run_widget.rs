use crate::clone;
use crate::quirky_app_context::QuirkyAppContext;
use crate::widget::Widget;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use futures_signals::signal_vec::MutableVecLockMut;
use futures_signals::signal_vec::VecDiff;
use futures_signals::signal_vec::{MutableVec, SignalVec, SignalVecExt};
use quirky_utils::futures_map_poll::FuturesMapPoll;
use std::collections::HashSet;
use std::future::Future;
use std::sync::Arc;
use uuid::Uuid;

pub fn run_widgets<'a>(
    ctx: &'a QuirkyAppContext,
    widgets_signal: impl SignalVec<Item = Arc<dyn Widget>> + Send + 'a,
) -> impl Future<Output = ()> + 'a {
    let widgets = MutableVec::new();
    let (widgets_futures_map, data) = FuturesMapPoll::new();

    let widgets_fut = widgets_signal.for_each(clone!(
        data,
        clone!(widgets, move |change: VecDiff<Arc<dyn Widget>>| {
            let mut widgets_lock = widgets.lock_mut();
            let mut widgets_futures_lock = data.lock().unwrap();

            MutableVecLockMut::<'_, _>::apply_vec_diff(&mut widgets_lock, change);

            // Add futures for newly inserted widgets
            for widget in widgets_lock.iter() {
                let id = widget.id();

                if !widgets_futures_lock.contains_key(&id) {
                    widgets_futures_lock.insert(id, widget.clone().run(ctx).boxed().into());
                }
            }

            let current_widget_ids: HashSet<Uuid> = widgets_lock.iter().map(|w| w.id()).collect();

            // Remove futures no longer in the widget list
            let ids_to_remove: Vec<Uuid> = widgets_futures_lock
                .iter()
                .filter(|w| !current_widget_ids.contains(w.0))
                .map(|w| *w.0)
                .collect();

            for id_to_remove in ids_to_remove {
                widgets_futures_lock.remove(&id_to_remove);
            }

            async move {}
        })
    ));

    let mut futs = FuturesUnordered::new();
    futs.push(widgets_fut.boxed());
    futs.push(widgets_futures_map.boxed());

    async move {
        loop {
            let _ = futs.select_next_some().await;
        }
    }
}
