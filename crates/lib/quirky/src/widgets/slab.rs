use crate::drawables::Drawable;
use crate::primitives::{Quad, Quads};
use crate::quirky_app_context::QuirkyAppContext;
use crate::widget::Widget;
use crate::{clone, LayoutBox, MouseEvent, SizeConstraint, WidgetEvent};
use async_std::task::sleep;
use async_trait::async_trait;
use futures::executor::block_on;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use futures_signals::signal::{always, ReadOnlyMutable, Signal};
use futures_signals::signal::{Mutable, SignalExt};
use futures_signals::signal_vec::MutableVec;
use glam::{uvec2, UVec2};
use quirky_macros::widget;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use wgpu::Device;

#[widget]
pub struct Slab {
    #[signal]
    #[default([0.02, 0.02, 0.02, 0.02])]
    color: [f32; 4],
    is_hovered: Mutable<bool>,
}

#[async_trait]
impl<
        ColorSignal: futures_signals::signal::Signal<Item = [f32; 4]> + Send + Sync + Unpin + 'static,
        ColorSignalFn: Fn() -> ColorSignal + Send + Sync,
    > Widget for Slab<ColorSignal, ColorSignalFn>
{
    fn id(&self) -> Uuid {
        self.id
    }

    fn paint(&self, device: &Device) -> Vec<Drawable> {
        let bb = self.bounding_box.get();

        if bb.size.x < 4 || bb.size.y < 4 {
            return vec![];
        }

        let color = if self.is_hovered.get() {
            [0.009, 0.009, 0.01, 1.0]
        } else {
            [0.005, 0.005, 0.005, 1.0]
        };

        vec![Drawable::Quad(Arc::new(Quads::new(
            vec![
                Quad::new(bb.pos, bb.size, [0.02, 0.02, 0.02, 1.0]),
                Quad::new(bb.pos + UVec2::new(1, 1), bb.size - UVec2::new(2, 2), color),
            ],
            device,
        )))]
    }

    fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send> {
        Box::new(always(SizeConstraint::MinSize(uvec2(10, 10))))
    }

    fn set_bounding_box(&self, new_box: LayoutBox) {
        self.bounding_box.set(new_box)
    }

    fn bounding_box(&self) -> ReadOnlyMutable<LayoutBox> {
        self.bounding_box.read_only()
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

    async fn run(
        self: Arc<Self>,
        ctx: &QuirkyAppContext,
        drawable_data: MutableVec<Drawable>,
        device: &Device,
    ) {
        let widget_events = ctx.subscribe_to_widget_events(self.id());
        let widget_events = futures_signals::signal::from_stream(widget_events);
        let redraw_counter = Mutable::new(0);

        let redraw_sig = redraw_counter
            .signal()
            .throttle(|| sleep(Duration::from_millis(20)))
            .for_each(clone!(
                self,
                clone!(drawable_data, move |_| {
                    drawable_data.lock_mut().replace_cloned(self.paint(device));
                    async move {}
                })
            ));

        let event_redraw = widget_events
            .throttle(|| sleep(Duration::from_millis(20)))
            .for_each(clone!(
                redraw_counter,
                clone!(self, move |e| {
                    if let Some(e) = e {
                        match e {
                            WidgetEvent::MouseEvent { event } => match event {
                                MouseEvent::Move { pos } => {
                                    self.is_hovered.set(true);
                                }
                                MouseEvent::Leave {} => {
                                    self.is_hovered.set(false);
                                }
                                _ => {}
                            },
                        }
                    }

                    async move {}
                })
            ));

        let hover_redraw =
            self.is_hovered
                .signal()
                .dedupe()
                .for_each(clone!(redraw_counter, move |_| {
                    redraw_counter.set(redraw_counter.get() + 1);
                    async move {}
                }));

        let bb_redraw = self
            .bounding_box
            .signal()
            .throttle(|| sleep(Duration::from_millis(20)))
            .for_each(|_| {
                redraw_counter.set(redraw_counter.get() + 1);
                async move {}
            });

        let mut futs = FuturesUnordered::new();

        futs.push(bb_redraw.boxed());
        futs.push(event_redraw.boxed());
        futs.push(hover_redraw.boxed());
        futs.push(redraw_sig.boxed());

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
