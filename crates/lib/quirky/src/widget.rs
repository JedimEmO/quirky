use crate::drawable_primitive::DrawablePrimitive;
use crate::quirky_app_context::QuirkyAppContext;
use crate::render_contexts::PrepareContext;
use crate::widgets::events::WidgetEvent;
use crate::LayoutBox;
use futures::{Stream, StreamExt};
use futures_signals::signal::{always, ReadOnlyMutable, Signal};
use glam::UVec2;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct Event {
    pub widget_event: WidgetEvent,
}

pub struct WidgetSettings {
    pub capture_events: bool,
}

pub trait WidgetBase {
    fn id(&self) -> Uuid;
    fn bounding_box(&self) -> ReadOnlyMutable<LayoutBox>;
    fn set_bounding_box(&self, new_box: LayoutBox);
    fn dirty(&self) -> ReadOnlyMutable<bool>;
    fn set_dirty(&self);
    fn clear_dirty(&self);
    fn get_cached_primitives(&self) -> Option<Vec<Box<dyn DrawablePrimitive>>>;
    fn set_cached_primitives(&self, primitives: Option<Vec<Box<dyn DrawablePrimitive>>>) -> ();
    fn poll_prop_futures<'a>(
        &'a self,
        ctx: &'a QuirkyAppContext,
    ) -> futures::stream::FuturesUnordered<futures::future::BoxFuture<'a, ()>>;
}

pub trait WidgetEventHandler {
    fn event_stream(&self) -> Box<dyn Stream<Item = Event>>;
}

#[async_trait::async_trait]
pub trait Widget: WidgetBase + Send + Sync {
    fn build(self) -> Arc<dyn Widget + 'static>
    where
        Self: Sized + 'static,
    {
        Arc::new(self) as Arc<dyn Widget + 'static>
    }

    fn children(&self) -> Option<Vec<Arc<dyn Widget>>> {
        None
    }

    fn prepare(
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
        self.children()
            .map(|children| {
                for child in children.iter().rev() {
                    if let Some(hit) = child.get_widget_at(_pos, _path.clone()) {
                        return Some(hit);
                    }
                }

                None
            })
            .flatten()
    }

    async fn run(self: Arc<Self>, ctx: &QuirkyAppContext);
}

pub async fn default_run(widget: Arc<dyn Widget>, ctx: &QuirkyAppContext) {
    let mut futs = widget.poll_prop_futures(ctx);

    loop {
        let _n = futs.select_next_some().await;
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SizeConstraint {
    MinSize(UVec2),
    MaxSize(UVec2),
    #[default]
    Unconstrained,
    MaxHeight(u32),
    MaxWidth(u32),
}
