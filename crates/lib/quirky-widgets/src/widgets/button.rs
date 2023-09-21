use crate::widgets::layout_item::{single_child_layout_strategy, LayoutItemBuilder};
use async_trait::async_trait;
use futures::stream::FuturesUnordered;
use futures::{select, FutureExt, StreamExt};
use futures_signals::signal::{always, Signal, SignalExt};
use glam::UVec2;
use quirky::primitives::quad::{Quad, Quads};
use quirky::primitives::{DrawablePrimitive, PrepareContext};
use quirky::quirky_app_context::QuirkyAppContext;
use quirky::styling::Padding;
use quirky::widget::{default_run, Widget, WidgetBase};
use quirky::{clone, layout, MouseButton, SizeConstraint};
use quirky_macros::widget;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Copy, Clone)]
pub struct ClickEvent {
    pub mouse_button: MouseButton,
}

#[widget]
pub struct Button {
    #[signal_prop]
    content: Arc<dyn Widget>,
    #[signal_prop]
    #[default(SizeConstraint::MaxHeight(32))]
    size_constraint: SizeConstraint,
    #[slot]
    on_click: ClickEvent,
}

#[async_trait]
impl<
        ContentSignal: futures_signals::signal::Signal<Item = Arc<dyn Widget>> + Send + Sync + Unpin + 'static,
        ContentSignalFn: Fn() -> ContentSignal + Send + Sync + 'static,
        SizeConstraintSignal: futures_signals::signal::Signal<Item = SizeConstraint> + Send + Sync + Unpin + 'static,
        SizeConstraintSignalFn: Fn() -> SizeConstraintSignal + Send + Sync + 'static,
        OnClickCallback: Fn(ClickEvent) -> () + Send + Sync,
    > Widget
    for Button<
        ContentSignal,
        ContentSignalFn,
        SizeConstraintSignal,
        SizeConstraintSignalFn,
        OnClickCallback,
    >
{
    fn build(self) -> Arc<dyn Widget> {
        Arc::new(self)
    }

    fn children(&self) -> Option<Vec<Arc<dyn Widget>>> {
        self.content_prop_value.get_cloned().map(|v| vec![v])
    }

    fn paint(
        &self,
        quirky_context: &QuirkyAppContext,
        _paint_ctx: &mut PrepareContext,
    ) -> Vec<Box<dyn DrawablePrimitive>> {
        let bb = self.bounding_box.get();

        let quads = Box::new(Quads::new(
            vec![Quad::new(bb.pos, bb.size, [0.02, 0.02, 0.02, 1.0])],
            &quirky_context.device,
        ));

        vec![quads]
    }

    fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send> {
        Box::new((self.size_constraint)())
    }

    fn get_widget_at(&self, pos: UVec2, mut path: Vec<Uuid>) -> Option<Vec<Uuid>> {
        let bb = self.bounding_box().get();

        if bb.contains(pos) {
            path.push(self.id);
            Some(path)
        } else {
            None
        }
    }

    async fn run(self: Arc<Self>, ctx: &QuirkyAppContext) {
        let mut futs = self.poll_prop_futures(ctx);

        let child_layouts = layout(
            self.bounding_box().signal(),
            self.content_prop_value
                .signal_cloned()
                .map(|v| v.into_iter().map(|c| c.size_constraint()).collect()),
            always(Padding {
                left: 10,
                right: 10,
                top: 5,
                bottom: 5,
            }),
            single_child_layout_strategy,
        )
        .for_each(clone!(self, move |new_layouts| {
            {
                self.content_prop_value.lock_ref().as_ref().map(|c| {
                    if let Some(l) = new_layouts.first() {
                        c.set_bounding_box(*l);
                    }
                });
                async move {}
            }
        }));

        let child_fut = self
            .content_prop_value
            .signal_cloned()
            .for_each(|c| async move {
                if let Some(c) = c {
                    c.run(ctx).await;
                }
            });

        futs.push(child_fut.boxed());
        futs.push(child_layouts.boxed());

        loop {
            futs.next().await;
        }
    }
}
