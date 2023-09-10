use crate::{EventDispatch, LayoutToken, WidgetEvent};
use async_std::prelude::Stream;
use futures::executor::block_on;
use futures::StreamExt;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use uuid::Uuid;

pub struct QuirkyAppContext {
    layouts_in_progress: Arc<AtomicI64>,
    event_stream_tx: async_broadcast::Sender<EventDispatch>,
    event_stream_rx: async_broadcast::Receiver<EventDispatch>,
}

impl QuirkyAppContext {
    pub fn new() -> Self {
        let (mut tx, rx) = async_broadcast::broadcast(5000);

        tx.set_overflow(true);

        Self {
            layouts_in_progress: Default::default(),
            event_stream_tx: tx,
            event_stream_rx: rx,
        }
    }

    pub fn dispatch_event(&self, target: Uuid, event: WidgetEvent) -> anyhow::Result<()> {
        block_on(self.event_stream_tx.broadcast(EventDispatch {
            receiver_id: target,
            event,
        }))?;
        Ok(())
    }

    pub fn subscribe_to_widget_events(
        &self,
        event_receiver: Uuid,
    ) -> impl Stream<Item = WidgetEvent> {
        self.event_stream_rx
            .clone()
            .filter(move |e| {
                let out = e.receiver_id == event_receiver;

                async move { out }
            })
            .map(|e| e.event)
    }

    pub fn start_layout(&self) -> LayoutToken {
        LayoutToken::new(self.layouts_in_progress.clone())
    }

    pub fn active_layouts(&self) -> i64 {
        self.layouts_in_progress.load(Ordering::Relaxed)
    }
}

impl Default for QuirkyAppContext {
    fn default() -> Self {
        Self::new()
    }
}
