use std::collections::HashMap;
use crate::{EventDispatch, LayoutToken, WidgetEvent};
use async_std::prelude::Stream;
use futures::channel::mpsc::channel;
use futures::StreamExt;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct QuirkyAppContext {
    layouts_in_progress: Arc<AtomicI64>,
    widget_event_subscriptions: Mutex<HashMap<Uuid, futures::channel::mpsc::Sender<WidgetEvent>>>
}

impl QuirkyAppContext {
    pub fn new() -> Self {
        Self {
            layouts_in_progress: Default::default(),
            widget_event_subscriptions: Default::default(),
        }
    }

    pub fn dispatch_event(&self, target: Uuid, event: WidgetEvent) -> anyhow::Result<()> {
        let mut sender_lock = self.widget_event_subscriptions.lock().unwrap();


        if let Some(sender) = sender_lock.get_mut(&target) {
            if sender.is_closed() {
                sender_lock.remove(&target);
            } else {
                sender.try_send(event)?;
            }
        }

        Ok(())
    }

    pub fn subscribe_to_widget_events(
        &self,
        event_receiver: Uuid,
    ) -> impl Stream<Item = WidgetEvent> {
        let (tx,rx) = channel(100);

        self.widget_event_subscriptions.lock().unwrap().insert(event_receiver, tx);

        rx
    }

    pub fn unsubscribe_from_widget_events(&self, widget_id: Uuid) {

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
