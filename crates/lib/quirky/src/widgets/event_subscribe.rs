use crate::quirky_app_context::QuirkyAppContext;
use crate::widget::Widget;
use crate::WidgetEvent;
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use std::future::Future;
use std::sync::Arc;

pub fn run_subscribe_to_events<'a, F: Future<Output = ()> + Send>(
    futs: FuturesUnordered<BoxFuture<'a, ()>>,
    widget: Arc<dyn Widget>,
    quirky_context: &'a QuirkyAppContext,
    event_handler: impl (Fn(WidgetEvent) -> F) + Send + Sync + 'a,
) -> FuturesUnordered<BoxFuture<'a, ()>> {
    let widget_events = quirky_context.subscribe_to_widget_events(widget.id());

    let events_fut = async move {
        let mut widget_events = widget_events.await.fuse();

        while let Some(evt) = widget_events.next().await {
            event_handler(evt).await;
        }
    };

    futs.push(events_fut.boxed());

    futs
}
