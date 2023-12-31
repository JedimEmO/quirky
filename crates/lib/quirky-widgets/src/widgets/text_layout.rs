use crate::primitives::text::TextRendererPrimitive;
use crate::resources::font_resource::FontResource;
use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use futures_signals::signal::{always, Signal, SignalExt};
use glam::{uvec2, UVec2};
use glyphon::{
    Attrs, Buffer, Color, Family, Metrics, Resolution, Shaping, TextArea, TextBounds, TextRenderer,
};
use quirky::drawable_primitive::DrawablePrimitive;
use quirky::quirky_app_context::QuirkyAppContext;
use quirky::render_contexts::PrepareContext;
use quirky::widget::SizeConstraint;
use quirky::widget::WidgetBase;
use quirky::widget::{Event, Widget};
use quirky_macros::widget;
use std::borrow::BorrowMut;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[widget]
pub struct TextLayout {
    #[signal_prop]
    #[default([1.0, 1.0, 1.0, 1.0])]
    color: [f32; 4],
    #[signal_prop]
    #[default("".into())]
    #[force_repaint]
    text: Arc<str>,
    #[slot]
    on_event: Event,
    text_buffer: Mutex<Option<Buffer>>,
}

#[async_trait]
impl<
        ColorSignal: Signal<Item = [f32; 4]> + Send + Sync + Unpin + 'static,
        ColorSignalFn: Fn() -> ColorSignal + Send + Sync + 'static,
        TextSignal: Signal<Item = Arc<str>> + Send + Sync + Unpin + 'static,
        TextSignalFn: Fn() -> TextSignal + Send + Sync + 'static,
        OnEventCallback: Fn(Event) -> () + Send + Sync,
    > Widget for TextLayout<ColorSignal, ColorSignalFn, TextSignal, TextSignalFn, OnEventCallback>
{
    fn prepare(
        &self,
        ctx: &QuirkyAppContext,
        paint_ctx: &mut PrepareContext,
    ) -> Vec<Box<dyn DrawablePrimitive>> {
        let font_resource = paint_ctx
            .resources
            .get_resource_mut::<FontResource>(std::any::TypeId::of::<FontResource>())
            .unwrap();

        let bb = self.bounding_box.get();
        let mut buffer_lock = self.text_buffer.lock().unwrap();

        let buffer = if let Some(mut buf) = buffer_lock.take() {
            buf.set_size(
                &mut font_resource.font_system,
                bb.size.x as f32,
                bb.size.y as f32,
            );

            buf.set_text(
                &mut font_resource.font_system,
                &self.text_prop_value.get_cloned().unwrap(),
                Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
            );
            buf.shape_until_scroll(&mut font_resource.font_system);
            buf
        } else {
            let mut buffer = Buffer::new(
                &mut font_resource.font_system,
                Metrics {
                    font_size: 15.0,
                    line_height: 17.0,
                },
            );

            buffer.set_size(
                &mut font_resource.font_system,
                bb.size.x as f32,
                bb.size.y as f32,
            );

            buffer.set_text(
                &mut font_resource.font_system,
                &self.text_prop_value.get_cloned().unwrap(),
                Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
            );

            buffer.shape_until_scroll(&mut font_resource.font_system);

            buffer
        };

        let mut renderer = TextRenderer::new(
            &mut font_resource.text_atlas.borrow_mut(),
            &ctx.device,
            Default::default(),
            None,
        );

        let screen_resolution = ctx.viewport_size.get();
        let buffer = buffer_lock.insert(buffer);

        let _ = renderer.prepare(
            &ctx.device,
            &ctx.queue,
            &mut font_resource.font_system.borrow_mut(),
            &mut font_resource.text_atlas.borrow_mut(),
            Resolution {
                width: screen_resolution.x,
                height: screen_resolution.y,
            },
            [TextArea {
                buffer,
                left: bb.pos.x as f32,
                top: bb.pos.y as f32,
                scale: 1.0,
                bounds: TextBounds {
                    left: bb.pos.x as i32,
                    top: bb.pos.y as i32,
                    right: (bb.pos.x as i32 + bb.size.x as i32).min(screen_resolution.x as i32),
                    bottom: (bb.pos.y as i32 + bb.size.y as i32).min(screen_resolution.y as i32),
                },
                default_color: Color::rgb(80, 80, 50),
            }],
            &mut font_resource.font_cache.borrow_mut(),
        );

        vec![Box::new(TextRendererPrimitive(renderer))]
    }

    fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send> {
        Box::new(always(SizeConstraint::MinSize(uvec2(10, 10))))
    }

    fn get_widget_at(&self, _pos: UVec2, _path: Vec<Uuid>) -> Option<Vec<Uuid>> {
        None
    }

    async fn run(self: Arc<Self>, ctx: &QuirkyAppContext) {
        let mut futs = self.poll_prop_futures(ctx);

        let bb_poll = self.bounding_box.signal().for_each(|_| {
            self.set_dirty();
            async move { ctx.signal_redraw().await }
        });

        futs.push(bb_poll.boxed());

        loop {
            let _n = futs.select_next_some().await;
        }
    }
}
