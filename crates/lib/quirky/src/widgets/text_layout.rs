use crate::primitives::DrawablePrimitive;
use crate::quirky_app_context::QuirkyAppContext;
use crate::widget::{Event, Widget};
use crate::widget::{PrepareContext, WidgetBase};
use crate::SizeConstraint;
use crate::{clone, LayoutBox};
use async_std::task::sleep;
use async_trait::async_trait;
use futures::executor::block_on;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use futures_signals::signal::{always, Mutable, Signal, SignalExt};
use futures_signals::signal_vec::MutableVec;
use glam::{uvec2, UVec2};
use glyphon::{
    Attrs, Buffer, Color, Family, Metrics, Resolution, Shaping, TextArea, TextBounds, TextRenderer,
};
use quirky_macros::widget;
use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use uuid::Uuid;
use wgpu::{Device, MultisampleState, Queue};

#[widget]
pub struct TextLayout {
    #[signal]
    #[default([1.0, 1.0, 1.0, 1.0])]
    color: [f32; 4],
    #[signal]
    #[default("".into())]
    text: Arc<str>,
    #[callback]
    on_event: Event,
    draw_color: RwLock<[f32; 4]>,
    #[default(RwLock::new("".into()))]
    text_content: RwLock<Arc<str>>,
    text_buffer: futures::lock::Mutex<Option<Buffer>>,
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
    fn paint(
        &self,
        ctx: &QuirkyAppContext,
        paint_ctx: &mut PrepareContext,
    ) -> Vec<Box<dyn DrawablePrimitive>> {
        let bb = self.bounding_box.get();
        let mut buffer_lock = block_on(self.text_buffer.lock());

        let buffer = if let Some(mut buf) = buffer_lock.take() {
            buf.set_size(
                &mut paint_ctx.font_system,
                bb.size.x as f32,
                bb.size.y as f32,
            );

            buf.set_text(
                &mut paint_ctx.font_system,
                &self.text_content.read().unwrap(),
                Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
            );
            buf.shape_until_scroll(&mut paint_ctx.font_system);
            buf
        } else {
            let mut buffer = Buffer::new(
                paint_ctx.font_system,
                Metrics {
                    font_size: 15.0,
                    line_height: 17.0,
                },
            );

            buffer.set_size(
                &mut paint_ctx.font_system,
                bb.size.x as f32,
                bb.size.y as f32,
            );

            buffer.set_text(
                &mut paint_ctx.font_system,
                &self.text_content.read().unwrap(),
                Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
            );

            buffer.shape_until_scroll(&mut paint_ctx.font_system);

            buffer
        };

        let mut renderer = TextRenderer::new(
            paint_ctx.text_atlas.borrow_mut(),
            &ctx.device,
            Default::default(),
            None,
        );

        let screen_resolution = ctx.viewport_size.get();
        let buffer = buffer_lock.insert(buffer);

        let _ = renderer.prepare(
            &ctx.device,
            &ctx.queue,
            paint_ctx.font_system.borrow_mut(),
            paint_ctx.text_atlas.borrow_mut(),
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
            paint_ctx.font_cache.borrow_mut(),
        );

        vec![Box::new(renderer)]
    }

    fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send> {
        Box::new(always(SizeConstraint::MinSize(uvec2(10, 10))))
    }

    fn get_widget_at(&self, _pos: UVec2, _path: Vec<Uuid>) -> Option<Vec<Uuid>> {
        None
    }

    async fn run(self: Arc<Self>, ctx: &QuirkyAppContext, device: &Device) {
        let mut futs = FuturesUnordered::new();
        let redraw_counter = Mutable::new(0);

        let redraw_sig = redraw_counter
            .signal()
            .throttle(|| sleep(Duration::from_millis(5)))
            .for_each(clone!(self, move |_| {
                self.set_dirty();
                async move {
                    ctx.signal_redraw().await;
                }
            }));

        let color_sig = (self.color)().for_each(clone!(
            self,
            clone!(redraw_counter, move |c| {
                *self.draw_color.write().unwrap() = c;
                async move {}
            })
        ));

        let str_sig = (self.text)().for_each(clone!(
            self,
            clone!(redraw_counter, move |txt| {
                clone!(
                    txt,
                    clone!(
                        self,
                        clone!(redraw_counter, async move {
                            *self.text_content.write().unwrap() = txt.clone();

                            redraw_counter.set(redraw_counter.get() + 1);
                        })
                    )
                )
            })
        ));

        let bb_redraw = self
            .bounding_box
            .signal()
            .throttle(|| sleep(Duration::from_millis(500)))
            .for_each(clone!(
                self,
                clone!(redraw_counter, move |bb| {
                    clone!(
                        self,
                        clone!(redraw_counter, async move {
                            redraw_counter.set(redraw_counter.get() + 1);
                        })
                    )
                })
            ));

        futs.push(redraw_sig.boxed());
        futs.push(color_sig.boxed());
        futs.push(str_sig.boxed());
        futs.push(bb_redraw.boxed());

        loop {
            let _n = futs.select_next_some().await;
        }
    }
}
