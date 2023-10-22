use crate::primitives::border_box::{BorderBox, BorderBoxData};
use crate::primitives::quad::{Quad, Quads};
use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use futures_signals::map_ref;
use futures_signals::signal::{Mutable, SignalExt};
use glam::UVec2;
use quirky::clone;
use quirky::drawable_primitive::DrawablePrimitive;
use quirky::quirky_app_context::QuirkyAppContext;
use quirky::render_contexts::PrepareContext;
use quirky::widget::{Widget, WidgetBase};
use quirky::widgets::event_subscribe::run_subscribe_to_events;
use quirky::widgets::events::{FocusState, KeyCode, KeyboardEvent, MouseEvent, WidgetEvent};
use quirky_macros::widget;
use std::sync::Arc;
use uuid::Uuid;

#[widget]
pub struct TextInput {
    #[signal_prop]
    text_value: String,
    #[slot]
    on_text_change: String,
    #[slot]
    on_focus_change: FocusState,
    #[slot]
    on_submit: (),
    #[default(Mutable::new(FocusState::Unfocused))]
    focus_state: Mutable<FocusState>,
    #[default(Mutable::new(Arc::new([])))]
    quad_geometry: Mutable<Arc<[Quad]>>,
    #[default(Default::default())]
    #[signal_prop]
    settings: TextInputSettings,
    border_box_data: Mutable<BorderBoxData>,
    hovered: Mutable<bool>,
}

impl<
        TextValueSignal: futures_signals::signal::Signal<Item = String> + Send + Sync + Unpin + 'static,
        TextValueSignalFn: Fn() -> TextValueSignal + Send + Sync + 'static,
        SettingsSignal: futures_signals::signal::Signal<Item = TextInputSettings> + Send + Sync + Unpin + 'static,
        SettingsSignalFn: Fn() -> SettingsSignal + Send + Sync + 'static,
        OnTextChangeCallback: Fn(String) -> () + Send + Sync + 'static,
        OnFocusChangeCallback: Fn(FocusState) -> () + Send + Sync + 'static,
        OnSubmitCallback: Fn(()) -> () + Send + Sync + 'static,
    >
    TextInput<
        TextValueSignal,
        TextValueSignalFn,
        SettingsSignal,
        SettingsSignalFn,
        OnTextChangeCallback,
        OnFocusChangeCallback,
        OnSubmitCallback,
    >
{
    fn regenerate_primitives(&self) {
        let bb = self.bounding_box.get();
        let settings = self.settings_prop_value.get().unwrap();

        let color = if self.hovered.get() {
            settings.background_color_hovered
        } else {
            settings.background_color
        };

        let border_color = if self.focus_state.get() == FocusState::Focused {
            settings.border_color_focused
        } else {
            settings.border_color
        };

        self.quad_geometry
            .set(Arc::new([Quad::new(bb.pos, bb.size, color)]));

        self.border_box_data.set(BorderBoxData {
            pos: *bb.pos.as_vec2().as_ref(),
            size: *bb.size.as_vec2().as_ref(),
            color: border_color,
            shade_color: [0.0, 0.0, 0.0, 0.0],
            border_side: 0,
            borders: [1, 1, 1, 1],
        })
    }
}

#[async_trait]
impl<
        TextValueSignal: futures_signals::signal::Signal<Item = String> + Send + Sync + Unpin + 'static,
        TextValueSignalFn: Fn() -> TextValueSignal + Send + Sync + 'static,
        SettingsSignal: futures_signals::signal::Signal<Item = TextInputSettings> + Send + Sync + Unpin + 'static,
        SettingsSignalFn: Fn() -> SettingsSignal + Send + Sync + 'static,
        OnTextChangeCallback: Fn(String) -> () + Send + Sync + 'static,
        OnFocusChangeCallback: Fn(FocusState) -> () + Send + Sync + 'static,
        OnSubmitCallback: Fn(()) -> () + Send + Sync + 'static,
    > Widget
    for TextInput<
        TextValueSignal,
        TextValueSignalFn,
        SettingsSignal,
        SettingsSignalFn,
        OnTextChangeCallback,
        OnFocusChangeCallback,
        OnSubmitCallback,
    >
{
    fn prepare(
        &self,
        quirky_context: &QuirkyAppContext,
        _paint_ctx: &mut PrepareContext,
    ) -> Vec<Box<dyn DrawablePrimitive>> {
        self.regenerate_primitives();

        let quads = Box::new(Quads::new(
            self.quad_geometry.read_only(),
            &quirky_context.device,
        ));

        let border_box = BorderBox::new(self.border_box_data.read_only(), &quirky_context.device);

        vec![quads, Box::new(border_box)]
    }

    fn get_widget_at(&self, pos: UVec2, mut path: Vec<Uuid>) -> Option<Vec<Uuid>> {
        if self.bounding_box.get().contains(pos) {
            path.push(self.id);
            Some(path)
        } else {
            None
        }
    }

    async fn run(self: Arc<Self>, ctx: &QuirkyAppContext) {
        let futs = self.poll_prop_futures(ctx);
        let update_sig = map_ref! {
            let _hovered = self.hovered.signal().dedupe(),
            let _focused = self.focus_state.signal(),
            let _bb = self.bounding_box.signal() => {
            }
        }
        .for_each(clone!(self, move |_| clone!(self, async move {
            self.regenerate_primitives();
            ctx.signal_redraw().await;
        })));

        let mut futs = run_subscribe_to_events(futs, self.clone(), ctx, |event| {
            match event {
                WidgetEvent::MouseEvent { event } => match event {
                    MouseEvent::ButtonUp { .. } => ctx.request_focus(self.id),
                    MouseEvent::Move { .. } => {
                        self.hovered.set(true);
                    }
                    MouseEvent::Leave { .. } => {
                        self.hovered.set(false);
                    }
                    _ => {}
                },
                WidgetEvent::KeyboardEvent { event } => match event {
                    KeyboardEvent::KeyPressed { key_code, modifier } => {
                        let char = if key_code < KeyCode::A && !modifier.shift {
                            if key_code == KeyCode::Key0 {
                                Some('0')
                            } else {
                                Some((b'1' + key_code as u8) as char)
                            }
                        } else if key_code < KeyCode::A && modifier.shift {
                            if key_code == KeyCode::Key1 {
                                Some('!')
                            } else if key_code == KeyCode::At {
                                Some('@')
                            } else if key_code == KeyCode::Key3 {
                                Some('#')
                            } else if key_code == KeyCode::Key4 {
                                Some('$')
                            } else if key_code == KeyCode::Key5 {
                                Some('%')
                            } else if key_code == KeyCode::Key6 {
                                Some('^')
                            } else if key_code == KeyCode::Key7 {
                                Some('&')
                            } else if key_code == KeyCode::Asterisk {
                                Some('*')
                            } else if key_code == KeyCode::Key9 {
                                Some('(')
                            } else if key_code == KeyCode::Key0 {
                                Some(')')
                            } else {
                                None
                            }
                        } else if key_code < KeyCode::Z && !modifier.shift {
                            Some((b'a' + key_code as u8 - KeyCode::A as u8) as char)
                        } else if key_code < KeyCode::Z && modifier.shift {
                            Some((b'A' + key_code as u8 - KeyCode::A as u8) as char)
                        } else if key_code == KeyCode::Space {
                            Some(' ')
                        } else if key_code == KeyCode::Comma {
                            Some(',')
                        } else if key_code == KeyCode::Period {
                            Some('.')
                        } else if key_code == KeyCode::At {
                            Some('@')
                        } else if key_code == KeyCode::Backslash {
                            Some('\\')
                        } else if key_code == KeyCode::Colon {
                            Some(':')
                        } else if key_code == KeyCode::Semicolon {
                            Some(';')
                        } else if key_code == KeyCode::Return {
                            (self.on_submit)(());
                            None
                        } else {
                            None
                        };

                        let current_value = self.text_value_prop_value.get_cloned().unwrap();

                        if let Some(new_char) = char {
                            (self.on_text_change)(format!("{}{}", current_value, new_char));
                        } else {
                            if key_code == KeyCode::Backspace && !current_value.is_empty() {
                                if modifier.ctrl {
                                    (self.on_text_change)("".to_string());
                                } else {
                                    (self.on_text_change)(
                                        current_value
                                            .chars()
                                            .take(current_value.len() - 1)
                                            .collect(),
                                    );
                                }
                            }
                        }
                    }
                },
                WidgetEvent::FocusChange(state) => {
                    self.focus_state.set(state);
                    (self.on_focus_change)(state);
                }
            }

            async move {}
        });

        futs.push(update_sig.boxed());

        loop {
            let _ = futs.select_next_some().await;
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TextInputSettings {
    pub background_color: [f32; 4],
    pub background_color_hovered: [f32; 4],
    pub border_color: [f32; 4],
    pub border_color_focused: [f32; 4],
}

impl Default for TextInputSettings {
    fn default() -> Self {
        Self {
            background_color: [0.002, 0.002, 0.002, 1.0],
            background_color_hovered: [0.002, 0.002, 0.002, 1.0],
            border_color: [0.02, 0.02, 0.02, 1.0],
            border_color_focused: [0.05, 0.05, 0.3, 1.0],
        }
    }
}
