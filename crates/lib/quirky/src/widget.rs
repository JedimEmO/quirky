use crate::{LayoutBox, SizeConstraint, WidgetEvent};
use futures_signals::signal::{always, ReadOnlyMutable, Signal};
use futures_signals::signal_vec::MutableVec;

use crate::primitives::DrawablePrimitive;
use crate::quirky_app_context::{FontContext, QuirkyAppContext};
use futures::Stream;
use glam::UVec2;
use glyphon::{FontSystem, SwashCache, TextAtlas};
use std::sync::Arc;
use uuid::Uuid;
use wgpu::{Device, Queue};

#[derive(Clone)]
pub struct Event {
    pub widget_event: WidgetEvent,
}

pub struct PrepareContext<'a> {
    pub font_system: &'a mut FontSystem,
    pub text_atlas: &'a mut TextAtlas,
    pub font_cache: &'a mut SwashCache,
}

pub trait WidgetBase {
    fn id(&self) -> Uuid;
    fn bounding_box(&self) -> ReadOnlyMutable<LayoutBox>;
    fn set_bounding_box(&self, new_box: LayoutBox);
    fn dirty(&self) -> ReadOnlyMutable<bool>;
    fn set_dirty(&self) -> ();
    fn clear_dirty(&self) -> ();
}

pub trait WidgetEventHandler {
    fn event_stream(&self) -> Box<dyn Stream<Item = Event>>;
}

#[async_trait::async_trait]
pub trait Widget: WidgetBase + Send + Sync {
    fn children(&self) -> Option<Vec<Arc<dyn Widget>>> {
        None
    }

    fn paint(
        &self,
        _quirky_context: &QuirkyAppContext,
        _paint_ctx: &mut PrepareContext,
    ) -> Vec<Box<dyn DrawablePrimitive>> {
        vec![]
    }

    fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send> {
        Box::new(always(SizeConstraint::Unconstrained))
    }

    fn get_widget_at(&self, _pos: UVec2, _path: Vec<Uuid>) -> Option<Vec<Uuid>> {
        None
    }

    async fn run(self: Arc<Self>, ctx: &QuirkyAppContext, device: &Device);
}
