use crate::primitives::quad::{Quad, Quads};
use crate::primitives::{DrawablePrimitive, PrepareContext};
use crate::quirky_app_context::QuirkyAppContext;
use crate::widget::{Event, Widget, WidgetBase};
use crate::{clone, LayoutBox, MouseEvent, SizeConstraint, WidgetEvent};
use async_std::task::sleep;
use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use futures_signals::signal::{always, Signal};
use futures_signals::signal::{Mutable, SignalExt};
use glam::{uvec2, UVec2};
use quirky_macros::widget;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

#[widget]
pub struct Slab {
    #[signal_prop]
    #[default([0.005, 0.005, 0.005, 1.0])]
    color: [f32; 4],
    #[signal_prop]
    #[default("".into())]
    text: Arc<str>,
    is_hovered: Mutable<bool>,
    #[callback]
    on_event: Event,
}

#[async_trait]
impl<
        ColorSignal: Signal<Item = [f32; 4]> + Send + Sync + Unpin + 'static,
        ColorSignalFn: Fn() -> ColorSignal + Send + Sync + 'static,
        TextSignal: Signal<Item = Arc<str>> + Send + Sync + Unpin + 'static,
        TextSignalFn: Fn() -> TextSignal + Send + Sync + 'static,
        OnEventCallback: Fn(Event) -> () + Send + Sync,
    > Widget for Slab<ColorSignal, ColorSignalFn, TextSignal, TextSignalFn, OnEventCallback>
{
    fn paint(
        &self,
        ctx: &QuirkyAppContext,
        _paint_ctx: &mut PrepareContext,
    ) -> Vec<Box<dyn DrawablePrimitive>> {
        let bb = self.bounding_box.get();

        if bb.size.x < 20 || bb.size.y < 20 {
            return vec![];
        }

        let color = if self.is_hovered.get() {
            [0.009, 0.009, 0.01, 1.0]
        } else {
            self.color_prop_value.get().unwrap()
        };

        let quads = Box::new(Quads::new(
            vec![
                Quad::new(bb.pos, bb.size, [0.02, 0.02, 0.02, 1.0]),
                Quad::new(bb.pos + UVec2::new(1, 1), bb.size - UVec2::new(2, 2), color),
            ],
            &ctx.device,
        ));

        vec![quads]
    }

    fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send> {
        Box::new(always(SizeConstraint::MinSize(uvec2(10, 10))))
    }

    fn get_widget_at(&self, pos: UVec2, mut path: Vec<Uuid>) -> Option<Vec<Uuid>> {
        let bb = self.bounding_box.get();

        if bb.contains(pos) {
            path.push(self.id());

            Some(path)
        } else {
            None
        }
    }

    async fn run(self: Arc<Self>, ctx: &QuirkyAppContext) {
        let widget_events = ctx.subscribe_to_widget_events(self.id());
        let widget_events = futures_signals::signal::from_stream(widget_events);

        let event_redraw = widget_events
            .throttle(|| sleep(Duration::from_millis(5)))
            .for_each(clone!(self, move |e| {
                clone!(self, async move {
                    if let Some(e) = e {
                        let ec = e.clone();
                        match e {
                            WidgetEvent::MouseEvent { event } => match event {
                                MouseEvent::Move { .. } => {
                                    self.is_hovered.set(true);
                                }
                                MouseEvent::Leave {} => {
                                    self.is_hovered.set(false);
                                }
                                MouseEvent::ButtonDown { .. } => {
                                    (self.on_event)(Event { widget_event: ec });
                                }
                                _ => {}
                            },
                        }
                    }
                })
            }));

        let hover_redraw = self.is_hovered.signal().dedupe().for_each(|_| {
            let ctx = &*ctx;
            self.set_dirty();
            async move {
                ctx.signal_redraw().await;
            }
        });

        let mut futs = self.poll_prop_futures(ctx);

        futs.push(event_redraw.boxed());
        futs.push(hover_redraw.boxed());

        loop {
            let _n = futs.select_next_some().await;
        }
    }
}

#[cfg(test)]
mod test {
    use crate::widgets::slab::SlabBuilder;
    use futures_signals::signal::always;

    #[test]
    fn slab_builder_test() {
        let _slab = SlabBuilder::new()
            .color_signal(|| always([0.0, 0.0, 0.0, 0.0]))
            .build();
    }
}
