use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use futures_signals::map_ref;
use futures_signals::signal::Signal;
use futures_signals::signal::{Mutable, SignalExt};
use glam::UVec2;
use quirky::primitives::quad::{Quad, Quads};
use quirky::primitives::{DrawablePrimitive, PrepareContext};
use quirky::quirky_app_context::QuirkyAppContext;
use quirky::widget::{Event, Widget, WidgetBase};
use quirky::widgets::event_subscribe::run_subscribe_to_events;
use quirky::SizeConstraint;
use quirky::{MouseEvent, WidgetEvent};
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
    #[default(Mutable::new(Arc::new([])))]
    quad_geometry: Mutable<Arc<[Quad]>>,
}

impl<
        ColorSignal: futures_signals::signal::Signal<Item = [f32; 4]> + Send + Sync + Unpin + 'static,
        ColorSignalFn: Fn() -> ColorSignal + Send + Sync + 'static,
        SizeConstraintSignal: futures_signals::signal::Signal<Item = SizeConstraint> + Send + Sync + Unpin + 'static,
        SizeConstraintSignalFn: Fn() -> SizeConstraintSignal + Send + Sync + 'static,
        OnEventCallback: Fn(Event) -> () + Send + Sync,
    >
    Slab<ColorSignal, ColorSignalFn, SizeConstraintSignal, SizeConstraintSignalFn, OnEventCallback>
{
    fn regenerate_primitives(&self) {
        let bb = self.bounding_box.get();
        let mut size = bb.size;

        if bb.size.length_squared() == 0 {
            size = UVec2::new(1, 1);
        }

        let color = if self.is_hovered.get() {
            [0.009, 0.009, 0.01, 1.0]
        } else {
            self.color_prop_value.get().unwrap()
        };

        self.quad_geometry
            .set(Arc::new([Quad::new(bb.pos, size, color)]));
    }
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
    fn prepare(
        &self,
        ctx: &QuirkyAppContext,
        _paint_ctx: &mut PrepareContext,
    ) -> Vec<Box<dyn DrawablePrimitive>> {
        self.regenerate_primitives();
        let quads = Box::new(Quads::new(self.quad_geometry.read_only(), &ctx.device));

        vec![quads]
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
        let regen_fut = map_ref! {
            let _bb = self.bounding_box.signal(),
            let _color = self.color_prop_value.signal(),
            let _hovered = self.is_hovered.signal() => {
            }
        }
        .for_each(|_| {
            self.regenerate_primitives();
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

        futs.push(regen_fut.boxed());

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
