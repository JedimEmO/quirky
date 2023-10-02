use crate::{FocusState, LayoutToken, MouseEvent, WidgetEvent};
use async_std::channel::Sender;
use async_std::prelude::Stream;
use async_std::sync::Mutex;
use futures::channel::mpsc::channel;
use futures_signals::signal::ReadOnlyMutable;
use glam::UVec2;
use glyphon::{FontSystem, SwashCache, TextAtlas};
use std::any::{Any, TypeId};
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

pub struct FontResource {
    pub font_system: FontSystem,
    pub font_cache: SwashCache,
    pub text_atlas: TextAtlas,
}

#[derive(Default)]
pub struct QuirkyResources {
    resources: HashMap<TypeId, Box<dyn Any + Send>>,
}

impl QuirkyResources {
    pub fn insert<T: Send + 'static>(&mut self, resource: T) {
        self.resources.insert(TypeId::of::<T>(), Box::new(resource));
    }

    pub fn get_resource<'a, T: 'static>(&'a self, type_id: TypeId) -> anyhow::Result<&'a T> {
        let resource = self
            .resources
            .get(&type_id)
            .expect(format!("Resource {:?} not found", type_id).as_str());

        resource
            .downcast_ref::<T>()
            .ok_or_else(|| anyhow::anyhow!("Resource type mismatch"))
    }

    pub fn get_resource_mut<T: 'static>(&mut self, type_id: TypeId) -> anyhow::Result<&mut T> {
        let resource = self
            .resources
            .get_mut(&type_id)
            .expect(format!("Resource {:?} not found", type_id).as_str());

        resource
            .downcast_mut::<T>()
            .ok_or_else(|| anyhow::anyhow!("Resource type mismatch"))
    }
}

pub struct QuirkyAppContext {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub viewport_size: ReadOnlyMutable<UVec2>,
    signal_dirty: Sender<()>,
    layouts_in_progress: Arc<AtomicI64>,
    widget_event_subscriptions:
        std::sync::Mutex<HashMap<Uuid, futures::channel::mpsc::Sender<WidgetEvent>>>,
    focused_widget_id: std::sync::Mutex<Option<Uuid>>,
}

impl QuirkyAppContext {
    pub fn new(
        device: Device,
        queue: Queue,
        viewport_size: ReadOnlyMutable<UVec2>,
        signal_dirty: Sender<()>,
    ) -> Self {
        Self {
            device: device.into(),
            queue: queue.into(),
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
        let mut sender_lock = self.widget_event_subscriptions.lock().unwrap();

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

    pub async fn subscribe_to_widget_events(
        &self,
        event_receiver: Uuid,
    ) -> impl Stream<Item = WidgetEvent> {
        let (tx, rx) = channel(1000);

        self.widget_event_subscriptions
            .lock()
            .unwrap()
            .insert(event_receiver, tx);

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
