use crate::primitives::border_box::{BorderBox, BorderBoxData};
use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use futures_signals::signal::Signal;
use futures_signals::signal::{Mutable, SignalExt};
use glam::UVec2;
use quirky::primitives::quad::{Quad, Quads};
use quirky::primitives::{DrawablePrimitive, PrepareContext};
use quirky::quirky_app_context::QuirkyAppContext;
use quirky::widget::{Event, Widget, WidgetBase};
use quirky::widgets::event_subscribe::run_subscribe_to_events;
use quirky::SizeConstraint;
use quirky::{clone, MouseEvent, WidgetEvent};
use quirky_macros::widget;
use std::sync::Arc;
use uuid::Uuid;

#[widget]
pub struct Slab {
    #[signal_prop]
    #[default([0.005, 0.005, 0.005, 1.0])]
    color: [f32; 4],
    #[signal_prop]
    #[default(SizeConstraint::Unconstrained)]
    pub size_constraint: SizeConstraint,
    is_hovered: Mutable<bool>,
    #[slot]
    on_event: Event,
}

#[async_trait]
impl<
        ColorSignal: futures_signals::signal::Signal<Item = [f32; 4]> + Send + Sync + Unpin + 'static,
        ColorSignalFn: Fn() -> ColorSignal + Send + Sync + 'static,
        SizeConstraintSignal: futures_signals::signal::Signal<Item = SizeConstraint> + Send + Sync + Unpin + 'static,
        SizeConstraintSignalFn: Fn() -> SizeConstraintSignal + Send + Sync + 'static,
        OnEventCallback: Fn(Event) -> () + Send + Sync,
    > Widget
    for Slab<
        ColorSignal,
        ColorSignalFn,
        SizeConstraintSignal,
        SizeConstraintSignalFn,
        OnEventCallback,
    >
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

        let geometry: Mutable<Arc<[Quad]>> = Mutable::new(Arc::new([
            Quad::new(bb.pos, bb.size, [0.02, 0.02, 0.02, 1.0]),
            Quad::new(bb.pos + UVec2::new(1, 1), bb.size - UVec2::new(2, 2), color),
        ]));

        let quads = Box::new(Quads::new(geometry.read_only(), &ctx.device));

        let data = Mutable::new(BorderBoxData {
            pos: *bb.pos.as_vec2().as_ref(),
            size: *bb.size.as_vec2().as_ref(),
            color: [0.02, 0.02, 0.02, 1.0],
            shade_color: [0.02, 0.02, 0.02, 1.0],
            border_side: 0,
            borders: [1, 1, 1, 1],
        });

        let border_box = Box::new(BorderBox::new(data.read_only(), &ctx.device));

        vec![quads, border_box]
    }

    fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send> {
        Box::new((self.size_constraint)())
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

        let event_redraw = widget_events.for_each(clone!(self, move |e| {
            clone!(self, async move {
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
                    _ => {}
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

        let futs = self.poll_prop_futures(ctx);

        let mut futs = run_subscribe_to_events(futs, self.clone(), ctx, |widget_event| {
            match widget_event.clone() {
                WidgetEvent::MouseEvent { event } => match event {
                    MouseEvent::Move { .. } => {
                        self.is_hovered.set(true);
                    }
                    MouseEvent::Leave {} => {
                        self.is_hovered.set(false);
                    }
                    MouseEvent::ButtonDown { .. } => {
                        (self.on_event)(Event { widget_event });
                    }
                    _ => {}
                },
                _ => {}
            }
            async move {}
        });
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
