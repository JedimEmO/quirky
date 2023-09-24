use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use futures_signals::signal::{Mutable, SignalExt};
use glam::UVec2;
use quirky::primitives::quad::{Quad, Quads};
use quirky::primitives::{DrawablePrimitive, PrepareContext};
use quirky::quirky_app_context::QuirkyAppContext;
use quirky::widget::{Widget, WidgetBase};
use quirky::widgets::event_subscribe::run_subscribe_to_events;
use quirky::{KeyCode, KeyboardEvent, MouseEvent, WidgetEvent};
use quirky_macros::widget;
use std::sync::Arc;
use uuid::Uuid;

#[widget]
pub struct TextInput {
    #[signal_prop]
    text_value: String,
    #[slot]
    on_text_change: String,
}

#[async_trait]
impl<
        TextValueSignal: futures_signals::signal::Signal<Item = String> + Send + Sync + Unpin + 'static,
        TextValueSignalFn: Fn() -> TextValueSignal + Send + Sync + 'static,
        OnTextChangeCallback: Fn(String) -> () + Send + Sync + 'static,
    > Widget for TextInput<TextValueSignal, TextValueSignalFn, OnTextChangeCallback>
{
    fn paint(
        &self,
        quirky_context: &QuirkyAppContext,
        _paint_ctx: &mut PrepareContext,
    ) -> Vec<Box<dyn DrawablePrimitive>> {
        let bb = self.bounding_box.get();
        let geometry: Mutable<Arc<[Quad]>> = Mutable::new(Arc::new([Quad::new(
            bb.pos,
            bb.size,
            [0.02, 0.1, 0.02, 1.0],
        )]));

        let quads = Box::new(Quads::new(geometry.read_only(), &quirky_context.device));
        vec![quads]
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
        let mut futs = run_subscribe_to_events(futs, self.clone(), ctx, |event| match event {
            WidgetEvent::MouseEvent { event } => match event {
                MouseEvent::ButtonUp { .. } => ctx.request_focus(self.id),
                _ => {}
            },
            WidgetEvent::KeyboardEvent { event } => match event {
                KeyboardEvent::KeyPressed { key_code, modifier } => {
                    let char = if key_code < KeyCode::A && !modifier.shift {
                        if key_code == KeyCode::Key0 {
                            Some('0')
                        } else {
                            Some(('1' as u8 + key_code as u8) as char)
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
                        Some(('a' as u8 + key_code as u8 - KeyCode::A as u8) as char)
                    } else if key_code < KeyCode::Z && modifier.shift {
                        Some(('A' as u8 + key_code as u8 - KeyCode::A as u8) as char)
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
                    } else {
                        None
                    };

                    let current_value = self.text_value_prop_value.get_cloned().unwrap();

                    if let Some(new_char) = char {
                        (self.on_text_change)(
                            format!("{}{}", current_value, new_char.to_string()).into(),
                        );
                    } else {
                        if key_code == KeyCode::Backspace && current_value.len() > 0 {
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
        });

        loop {
            let _ = futs.select_next_some().await;
        }
    }
}
