use async_trait::async_trait;
use futures::FutureExt;
use futures_signals::signal::SignalExt;
use glam::UVec2;
use quirky::primitives::{DrawablePrimitive, PrepareContext};
use quirky::quirky_app_context::QuirkyAppContext;
use quirky::widget::Widget;
use quirky_macros::widget;
use std::sync::Arc;
use uuid::Uuid;

#[widget]
pub struct TextInput {
    #[signal_prop]
    text_value: Arc<str>,
    #[slot]
    on_text_change: Arc<str>,
}

#[async_trait]
impl<
        TextValueSignal: futures_signals::signal::Signal<Item = Arc<str>> + Send + Sync + Unpin + 'static,
        TextValueSignalFn: Fn() -> TextValueSignal + Send + Sync + 'static,
        OnTextChangeCallback: Fn(Arc<str>) -> () + Send + Sync + 'static,
    > Widget for TextInput<TextValueSignal, TextValueSignalFn, OnTextChangeCallback>
{
    fn children(&self) -> Option<Vec<Arc<dyn Widget>>> {
        todo!()
    }

    fn paint(
        &self,
        _quirky_context: &QuirkyAppContext,
        _paint_ctx: &mut PrepareContext,
    ) -> Vec<Box<dyn DrawablePrimitive>> {
        todo!()
    }

    fn get_widget_at(&self, _pos: UVec2, _path: Vec<Uuid>) -> Option<Vec<Uuid>> {
        todo!()
    }

    async fn run(self: Arc<Self>, ctx: &QuirkyAppContext) {
        todo!()
    }
}
