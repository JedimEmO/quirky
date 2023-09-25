use crate::{FocusState, LayoutToken, MouseEvent, WidgetEvent};
use async_std::channel::Sender;
use async_std::prelude::Stream;
use async_std::sync::Mutex;
use futures::channel::mpsc::channel;
use futures::executor::block_on;
use futures_signals::signal::ReadOnlyMutable;
use glam::UVec2;
use glyphon::{FontSystem, SwashCache, TextAtlas};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use uuid::Uuid;
use wgpu::{Device, Queue};

pub struct FontContext {
    pub font_system: Mutex<FontSystem>,
    pub font_cache: Mutex<SwashCache>,
    pub text_atlas: Mutex<TextAtlas>,
}

pub struct QuirkyAppContext {
    pub font_context: FontContext,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub viewport_size: ReadOnlyMutable<UVec2>,
    signal_dirty: Sender<()>,
    layouts_in_progress: Arc<AtomicI64>,
    widget_event_subscriptions: Mutex<HashMap<Uuid, futures::channel::mpsc::Sender<WidgetEvent>>>,
    focused_widget_id: std::sync::Mutex<Option<Uuid>>,
}

impl QuirkyAppContext {
    pub fn new(
        device: Device,
        queue: Queue,
        font_context: FontContext,
        viewport_size: ReadOnlyMutable<UVec2>,
        signal_dirty: Sender<()>,
    ) -> Self {
        Self {
            device: device.into(),
            queue: queue.into(),
            font_context,
            layouts_in_progress: Default::default(),
            widget_event_subscriptions: Default::default(),
            viewport_size,
            signal_dirty,
            focused_widget_id: Default::default(),
        }
    }

    pub async fn signal_redraw(&self) {
        self.signal_dirty.send(()).await.unwrap();
    }

    pub fn dispatch_event(&self, mut target: Uuid, event: WidgetEvent) -> anyhow::Result<()> {
        let mut sender_lock = block_on(self.widget_event_subscriptions.lock());

        if let WidgetEvent::KeyboardEvent { .. } = &event {
            if let Some(locked_id) = self.focused_widget_id.lock().unwrap().as_ref() {
                target = *locked_id;
            }
        }

        if let WidgetEvent::MouseEvent { event } = &event {
            if let MouseEvent::ButtonDown { .. } = event {
                let mut currently_focused = self.focused_widget_id.lock().unwrap();

                if Some(target) != *currently_focused && currently_focused.is_some() {
                    if let Some(sender) = sender_lock.get_mut(&currently_focused.unwrap()) {
                        if sender.is_closed() {
                            sender_lock.remove(&target);
                        } else {
                            sender.try_send(WidgetEvent::FocusChange(FocusState::Unfocused))?;
                        }
                    }
                }

                currently_focused.take();
            }
        };

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
        let (tx, rx) = channel(1000);

        block_on(self.widget_event_subscriptions.lock()).insert(event_receiver, tx);

        rx
    }

    pub fn unsubscribe_from_widget_events(&self, _widget_id: Uuid) {}

    pub fn start_layout(&self) -> LayoutToken {
        LayoutToken::new(self.layouts_in_progress.clone())
    }

    pub fn active_layouts(&self) -> i64 {
        self.layouts_in_progress.load(Ordering::Relaxed)
    }

    pub fn request_focus(&self, widget_id: Uuid) {
        let _ = self.focused_widget_id.lock().unwrap().insert(widget_id);
        let _ = self.dispatch_event(widget_id, WidgetEvent::FocusChange(FocusState::Focused));
    }
}
