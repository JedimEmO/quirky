use crate::drawables::Drawable;
use crate::{LayoutBox, SizeConstraint};
use futures_signals::signal::{ReadOnlyMutable, Signal};
use futures_signals::signal_vec::MutableVec;

use std::sync::Arc;
use wgpu::Device;

#[async_trait::async_trait]
pub trait Widget: Send + Sync {
    fn paint(&self, device: &Device) -> Vec<Drawable>;
    fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send>;
    fn set_bounding_box(&self, new_box: LayoutBox);
    fn bounding_box(&self) -> ReadOnlyMutable<LayoutBox>;
    async fn run(self: Arc<Self>, drawable_data: MutableVec<Drawable>, device: &Device);
}

pub mod widgets {
    use crate::drawables::Drawable;

    use crate::primitives::{Quad, Quads};
    use crate::widget::Widget;
    use crate::{layout, LayoutBox, SizeConstraint};
    use async_trait::async_trait;
    use futures::select;
    use futures::stream::FuturesUnordered;
    use futures::{FutureExt, StreamExt};
    use futures_signals::signal::{always, Mutable, ReadOnlyMutable, Signal, SignalExt};
    use futures_signals::signal_vec::MutableVec;
    use glam::{uvec2, UVec2};
    use std::sync::Arc;
    use wgpu::Device;

    #[derive(Debug, Default)]
    pub struct Slab {
        pub bounding_box: Mutable<LayoutBox>,
    }

    #[async_trait]
    impl Widget for Slab {
        fn paint(&self, device: &Device) -> Vec<Drawable> {
            let bb = self.bounding_box.get();

            vec![
                Drawable::Quad(Arc::new(Quads::new(
                    vec![Quad::new(bb.pos, bb.size, [0.2, 0.2, 0.2, 1.0])],
                    device,
                ))),
                Drawable::Quad(Arc::new(Quads::new(
                    vec![Quad::new(
                        bb.pos + UVec2::new(2, 2),
                        bb.size - UVec2::new(4, 4),
                        [0.3, 0.3, 0.3, 1.0],
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

        async fn run(self: Arc<Self>, drawable_data: MutableVec<Drawable>, device: &Device) {
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

    #[derive(Default)]
    pub struct List {
        pub children: Mutable<Vec<Arc<dyn Widget>>>,
        pub requested_size: Mutable<SizeConstraint>,
        pub bounding_box: Mutable<LayoutBox>,
        pub background: Option<[f32; 4]>,
    }

    #[async_trait]
    impl Widget for List {
        fn paint(&self, device: &Device) -> Vec<Drawable> {
            if let Some(background) = self.background {
                let bb = self.bounding_box().get();
                vec![Drawable::Quad(Arc::new(Quads::new(
                    vec![Quad::new(bb.pos, bb.size, background)],
                    device,
                )))]
            } else {
                vec![]
            }
        }

        fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send> {
            Box::new(self.requested_size.signal_cloned())
        }

        fn set_bounding_box(&self, new_box: LayoutBox) {
            self.bounding_box.set(new_box)
        }

        fn bounding_box(&self) -> ReadOnlyMutable<LayoutBox> {
            self.bounding_box.read_only()
        }

        async fn run(self: Arc<Self>, drawable_data: MutableVec<Drawable>, device: &Device) {
            let child_layouts = layout(
                self.bounding_box().signal(),
                self.children
                    .signal_cloned()
                    .map(|v| v.into_iter().map(|c| c.size_constraint()).collect()),
                |a, b| {
                    let total_items = b.len().max(1) as u32;

                    let item_heights = (a.size.y - b.len().max(1) as u32 * 5 + 5) / total_items;

                    (0..b.len())
                        .map(|i| LayoutBox {
                            pos: a.pos + UVec2::new(0, (i * item_heights as usize + i * 5) as u32),
                            size: UVec2::new(a.size.x, item_heights),
                        })
                        .collect()
                },
            );

            let mut child_layouts_stream = child_layouts.to_stream().fuse();
            let mut child_run_futs = FuturesUnordered::new();

            loop {
                let mut next_layouts = child_layouts_stream.select_next_some();
                let mut next_child_run_fut = child_run_futs.select_next_some();

                select! {
                    layouts = next_layouts => {
                         child_run_futs = FuturesUnordered::new();

                        let mut new_drawables = self.paint(device);

                        layouts.iter().enumerate().for_each(|(idx, l)| {
                            let child = self.children.lock_ref()[idx].clone();

                            child.set_bounding_box(*l);
                            let child_subtree = MutableVec::new_with_values(child.paint(device));
                            new_drawables.push(Drawable::SubTree {children: child_subtree.clone(), transform: l.pos, size: l.size });
                            child_run_futs.push(child.run(child_subtree, device));
                        });

                        drawable_data.lock_mut().replace_cloned(new_drawables);
                    }

                    _childruns = next_child_run_fut => {}
                }
            }
        }
    }
}
