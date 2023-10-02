use crate::layouts::anchored_container::{single_child_layout_strategy, AnchorPoint};
use crate::primitives::button_primitive::{ButtonData, ButtonPrimitive};
use crate::styling::Padding;
use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use futures_signals::signal::{always, Mutable, Signal, SignalExt};
use glam::UVec2;
use quirky::clone;
use quirky::drawable_primitive::DrawablePrimitive;
use quirky::quirky_app_context::QuirkyAppContext;
use quirky::render_contexts::PrepareContext;
use quirky::widget::{SizeConstraint, Widget, WidgetBase};
use quirky::widgets::event_subscribe::run_subscribe_to_events;
use quirky::widgets::events::{MouseButton, MouseEvent, WidgetEvent};
use quirky::widgets::layout_helper::layout;
use quirky_macros::widget;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Copy, Clone)]
pub struct ClickEvent {
    pub mouse_button: MouseButton,
}

#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub enum ButtonState {
    Hovered,
    Pressed,
    #[default]
    Default,
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
    #[slot]
    on_button_state_change: ButtonState,
    button_state: Mutable<ButtonState>,
    #[default(Mutable::new(Default::default()))]
    button_data: Mutable<ButtonData>,
}

#[async_trait]
impl<
        ContentSignal: futures_signals::signal::Signal<Item = Arc<dyn Widget>> + Send + Sync + Unpin + 'static,
        ContentSignalFn: Fn() -> ContentSignal + Send + Sync + 'static,
        SizeConstraintSignal: futures_signals::signal::Signal<Item = SizeConstraint> + Send + Sync + Unpin + 'static,
        SizeConstraintSignalFn: Fn() -> SizeConstraintSignal + Send + Sync + 'static,
        OnClickCallback: Fn(ClickEvent) -> () + Send + Sync + 'static,
        OnButtonStateChangeCallback: Fn(ButtonState) -> () + Send + Sync + 'static,
    > Widget
    for Button<
        ContentSignal,
        ContentSignalFn,
        SizeConstraintSignal,
        SizeConstraintSignalFn,
        OnClickCallback,
        OnButtonStateChangeCallback,
    >
{
    fn build(self) -> Arc<dyn Widget> {
        Arc::new(self)
    }

    fn children(&self) -> Option<Vec<Arc<dyn Widget>>> {
        self.content_prop_value.get_cloned().map(|v| vec![v])
    }

    fn prepare(
        &self,
        quirky_context: &QuirkyAppContext,
        _paint_ctx: &mut PrepareContext,
    ) -> Vec<Box<dyn DrawablePrimitive>> {
        let button_primitive =
            ButtonPrimitive::new(self.button_data.read_only(), &quirky_context.device);

        vec![Box::new(button_primitive)]
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
        let futs = self.poll_prop_futures(ctx);
        let state_change_fut = self.button_state.signal().for_each(|data| {
            let color = match data {
                ButtonState::Default => [0.03, 0.03, 0.03, 1.0],
                ButtonState::Hovered => [0.02, 0.02, 0.02, 1.0],
                ButtonState::Pressed => [0.01, 0.01, 0.01, 0.8],
            };

            let mut data = self.button_data.get();
            data.color = color;
            self.button_data.set(data);

            async move { ctx.signal_redraw().await }
        });

        let bb_change_fut = self.bounding_box.signal().for_each(|new_bb| {
            let mut data = self.button_data.get();
            data.pos = *new_bb.pos.as_vec2().as_ref();
            data.size = *new_bb.size.as_vec2().as_ref();
            self.button_data.set(data);

            async move { ctx.signal_redraw().await }
        });

        let mut futs = run_subscribe_to_events(
            futs,
            self.clone(),
            ctx,
            clone!(self, move |widget_event| {
                match widget_event.clone() {
                    WidgetEvent::MouseEvent { event } => match event {
                        MouseEvent::Move { .. } => {
                            self.button_state.set(ButtonState::Hovered);
                        }
                        MouseEvent::ButtonDown { .. } => {
                            self.button_state.set(ButtonState::Pressed);
                        }
                        MouseEvent::ButtonUp { button } => {
                            self.button_state.set(ButtonState::Hovered);
                            (self.on_click)(ClickEvent {
                                mouse_button: button,
                            })
                        }
                        _ => {
                            self.button_state.set(ButtonState::Default);
                        }
                    },
                    _ => {}
                }
                async move {}
            }),
        );

        let child_layouts = layout(
            self.bounding_box().signal(),
            self.content_prop_value
                .signal_cloned()
                .map(|v| v.into_iter().map(|c| c.size_constraint()).collect())
                .to_signal_vec(),
            always((
                Padding {
                    left: 3,
                    right: 3,
                    top: 3,
                    bottom: 3,
                },
                AnchorPoint::Center,
            )),
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
        futs.push(state_change_fut.boxed());
        futs.push(bb_change_fut.boxed());

        loop {
            futs.next().await;
        }
    }
}
