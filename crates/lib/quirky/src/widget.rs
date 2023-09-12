use crate::drawables::Drawable;
use crate::{LayoutBox, SizeConstraint, WidgetEvent};
use futures_signals::signal::{always, ReadOnlyMutable, Signal};
use futures_signals::signal_vec::MutableVec;

use crate::quirky_app_context::QuirkyAppContext;
use glam::UVec2;
use std::sync::Arc;
use futures::Stream;
use uuid::Uuid;
use wgpu::Device;

#[derive(Clone)]
pub struct Event {
    pub widget_event: WidgetEvent
}

pub trait WidgetBase {
    fn id(&self) -> Uuid;
    fn bounding_box(&self) -> ReadOnlyMutable<LayoutBox>;
    fn set_bounding_box(&self, new_box: LayoutBox);
}

pub trait WidgetEventHandler {
    fn event_stream(&self) -> Box<dyn Stream<Item=Event>>;
}

#[async_trait::async_trait]
pub trait Widget: WidgetBase + Send + Sync {
    fn paint(&self, _device: &Device) -> Vec<Drawable> {
        vec![]
    }

    fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send> {
        Box::new(always(SizeConstraint::Unconstrained))
    }

    fn get_widget_at(&self, _pos: UVec2, _path: Vec<Uuid>) -> Option<Vec<Uuid>> {
        None
    }

    async fn run(
        self: Arc<Self>,
        ctx: &QuirkyAppContext,
        drawable_data: MutableVec<Drawable>,
        device: &Device,
    );
}
