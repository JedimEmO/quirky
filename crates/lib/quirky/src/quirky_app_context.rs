use std::collections::HashMap;
use crate::{LayoutToken, WidgetEvent};
use async_std::prelude::Stream;
use futures::channel::mpsc::channel;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use futures_signals::signal::ReadOnlyMutable;
use glam::UVec2;
use glyphon::{FontSystem, SwashCache, TextAtlas};
use uuid::Uuid;
use wgpu::{Device, Queue};
use crate::ui_camera::UiCamera2D;

pub struct FontContext {
    pub font_system: Mutex<FontSystem>,
    pub font_cache: Mutex<SwashCache>,
    pub text_atlas: Mutex<TextAtlas>
}

pub struct QuirkyAppContext {
    pub font_context: FontContext,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub viewport_size: ReadOnlyMutable<UVec2>,
    layouts_in_progress: Arc<AtomicI64>,
    widget_event_subscriptions: Mutex<HashMap<Uuid, futures::channel::mpsc::Sender<WidgetEvent>>>
}

impl QuirkyAppContext {
    pub fn new(device: Device, queue: Queue, font_context: FontContext, viewport_size: ReadOnlyMutable<UVec2>) -> Self {
        Self {
            device: device.into(),
            queue: queue.into(),
            font_context,
            layouts_in_progress: Default::default(),
            widget_event_subscriptions: Default::default(),
            viewport_size,
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
        let (tx,rx) = channel(1000);

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

