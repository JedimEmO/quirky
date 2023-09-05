use std::fmt::Debug;
use futures_signals::signal::{ReadOnlyMutable, Signal};
use std::sync::Arc;
use futures_signals::signal_vec::MutableVec;
use wgpu::Device;
use crate::drawables::Drawable;
use crate::{LayoutBox, SizeConstraint};

#[async_trait::async_trait]
pub trait Widget: Sync + Send + Debug {
    fn paint(&self, device: &Device) -> Vec<Drawable>;
    fn size_constraint(&self) -> Box<dyn Signal<Item=SizeConstraint> + Unpin + Send>;
    fn set_bounding_box(&self, new_box: LayoutBox) -> ();
    fn bounding_box(&self) -> ReadOnlyMutable<LayoutBox>;
    async fn run(self: Arc<Self>, drawable_data: MutableVec<Drawable>, device: &Device);
}

pub mod widgets {
    use crate::drawables::Drawable;

    use crate::{layout, LayoutBox, SizeConstraint};
    use async_trait::async_trait;
    use futures::select;
    use futures::{FutureExt, StreamExt};
    use futures_signals::signal::{always, Mutable, ReadOnlyMutable, Signal, SignalExt};
    use futures_signals::signal_vec::{MutableVec, SignalVecExt};
    use glam::{uvec2, UVec2};
    use std::sync::Arc;
    use futures::stream::FuturesUnordered;
    use wgpu::Device;
    use crate::primitives::{Quad, Quads};
    use crate::widget::Widget;

    #[derive(Debug, Default)]
    pub struct Slab {
        pub bounding_box: Mutable<LayoutBox>,
    }

    #[async_trait]
    impl Widget for Slab {
        fn paint(&self, device: &Device) -> Vec<Drawable> {
            vec![]
        }

        fn size_constraint(&self) -> Box<dyn Signal<Item=SizeConstraint> + Unpin + Send> {
            Box::new(always(SizeConstraint::MinSize(uvec2(10, 10))))
        }

        fn set_bounding_box(&self, new_box: LayoutBox) -> () {
            self.bounding_box.set(new_box)
        }

        fn bounding_box(&self) -> ReadOnlyMutable<LayoutBox> {
            self.bounding_box.read_only()
        }

        async fn run(self: Arc<Self>, drawable_data: MutableVec<Drawable>, device: &Device) {
            self.bounding_box.signal().to_stream().for_each(|bb| {
                drawable_data.lock_mut().replace_cloned(vec![Drawable::Quad(Arc::new(Quads::new(vec![Quad::new( bb.pos, bb.size, [0.3, 0.3, 0.5, 1.0])], device)))]);
                async move {}
            }).await;
        }
    }

    #[derive(Default, Debug)]
    pub struct List {
        pub children: Mutable<Vec<Arc<dyn Widget>>>,
        pub requested_size: Mutable<SizeConstraint>,
        pub bounding_box: Mutable<LayoutBox>,
    }

    #[async_trait]
    impl Widget for List {
        fn paint(&self, device: &Device) -> Vec<Drawable> {
            vec![]
        }

        fn size_constraint(&self) -> Box<dyn Signal<Item=SizeConstraint> + Unpin + Send> {
            Box::new(self.requested_size.signal_cloned())
        }

        fn set_bounding_box(&self, new_box: LayoutBox) -> () {
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
                            size: UVec2::new(100, item_heights),
                        })
                        .collect()
                },
            );

            let mut child_layouts_stream = child_layouts.to_stream();
            let mut child_run_futs = FuturesUnordered::new();

            loop {
                let mut next_layouts = child_layouts_stream.next().fuse();
                let mut next_child_run_fut = child_run_futs.next();

                select! {
                    layouts = next_layouts => {
                        if let Some(layouts) = layouts {
                            child_run_futs = FuturesUnordered::new();

                            let mut new_drawables = vec![];

                            layouts.iter().enumerate().for_each(|(idx, l)| {
                                let child = self.children.lock_ref()[idx].clone();

                                child.set_bounding_box(*l);
                                let child_subtree = MutableVec::new();
                                new_drawables.push(Drawable::SubTree {children: child_subtree.clone(), transform: l.pos, size: l.size });
                                child_run_futs.push(child.run(child_subtree, device));
                            });

                            drawable_data.lock_mut().replace_cloned(new_drawables);
                        }
                    }

                    _childruns = next_child_run_fut => {}
                }
            }
        }
    }
}
