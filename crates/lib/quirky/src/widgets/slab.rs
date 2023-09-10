use std::sync::Arc;
use async_trait::async_trait;
use futures_signals::signal::{always, Mutable, ReadOnlyMutable, Signal};
use futures_signals::signal_vec::MutableVec;
use futures_signals::signal::SignalExt;
use futures::StreamExt;
use glam::{UVec2, uvec2};
use wgpu::Device;
use quirky_macros::widget;
use crate::drawables::Drawable;
use crate::{LayoutBox, QuirkyAppContext, SizeConstraint};
use crate::primitives::{Quad, Quads};
use crate::widget::Widget;

#[widget]
pub struct Slab {
    #[signal]
    #[default([0.02, 0.02, 0.02, 0.02])]
    color: [f32; 4],
}

#[async_trait]
impl<ColorSignal: futures_signals::signal::Signal<Item=[f32; 4]> + Send + Sync + 'static, ColorSignalFn: Fn() -> ColorSignal + Send + Sync> Widget for Slab<ColorSignal, ColorSignalFn> {
    fn paint(&self, device: &Device) -> Vec<Drawable> {
        let bb = self.bounding_box.get();

        if bb.size.x < 4 || bb.size.y < 4 {
            return vec![];
        }

        vec![
            Drawable::Quad(Arc::new(Quads::new(
                vec![Quad::new(bb.pos, bb.size, [0.02, 0.02, 0.02, 1.0])],
                device,
            ))),
            Drawable::Quad(Arc::new(Quads::new(
                vec![Quad::new(
                    bb.pos + UVec2::new(1, 1),
                    bb.size - UVec2::new(2, 2),
                    [0.005, 0.005, 0.005, 1.0],
                )],
                device,
            ))),
        ]
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

    async fn run(
        self: Arc<Self>,
        _ctx: &QuirkyAppContext,
        drawable_data: MutableVec<Drawable>,
        device: &Device,
    ) {
        self.bounding_box
            .signal()
            .to_stream()
            .for_each(|_bb| {
                drawable_data.lock_mut().replace_cloned(self.paint(device));
                async move {}
            })
            .await;
    }
}

#[cfg(test)]
mod test {
    use futures_signals::signal::always;
    use crate::widgets::slab::SlabBuilder;

    #[test]
    fn slab_builder_test() {
        let _slab = SlabBuilder::new()
            .color_signal(|| always([0.0, 0.0, 0.0, 0.0]))
            .build();
    }
}