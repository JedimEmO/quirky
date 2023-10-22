use crate::layouts::anchored_container::{AnchorPoint, AnchoredContainerBuilder};
use crate::styling::Padding;
use crate::theming::QuirkyTheme;
use crate::widgets::label::{FontSettings, LabelBuilder};
use crate::widgets::stack::StackBuilder;
use crate::widgets::text_input::{TextInputBuilder, TextInputSettings};
use futures_signals::map_ref;
use futures_signals::signal::{Mutable, Signal, SignalExt};
use futures_signals::signal_vec::MutableVec;
use glyphon::{FamilyOwned, Metrics, Style, Weight};
use quirky::clone;
use quirky::widget::{SizeConstraint, Widget};
use quirky::widgets::events::FocusState;
use std::sync::Arc;

pub fn text_input(
    value_signal: impl Signal<Item = String> + Send + Sync + 'static,
    on_value: impl Fn(String) -> () + Send + Sync + 'static,
    on_submit: impl Fn(()) -> () + Send + Sync + 'static,
    theme: &QuirkyTheme,
) -> Arc<dyn Widget> {
    let theme = theme;
    let value_bc = value_signal.broadcast();
    let is_focused = Mutable::new(false);

    let text_settings = TextInputSettings {
        background_color: theme.input.background_color,
        background_color_hovered: theme.input.background_color_hovered,
        border_color: theme.input.border_color,
        border_color_focused: theme.input.border_color_focused,
    };

    let label_value = Mutable::new("Some label".to_string());

    let label = map_ref! {
        let label = label_value.signal_cloned(),
        let value = value_bc.signal_cloned() => {
            if value.len() < 5 {
                label.clone()
            } else {
                "The value is 5 or more characters".to_string()
            }
        }
    }
    .broadcast();

    let lift_label = map_ref! {
        let focused = is_focused.signal(),
        let empty = value_bc.signal_cloned().map(|v| v.is_empty()) => {
            *focused || !*empty
        }
    }
    .broadcast();

    let label_color = theme.input.label_color;
    let label_color_invalid = theme.input.label_color_invalid;

    let children = MutableVec::new_with_values(vec![
        TextInputBuilder::new()
            .text_value_signal(clone!(value_bc, move || value_bc.signal_cloned()))
            .on_text_change(on_value)
            .on_submit(on_submit)
            .settings(text_settings)
            .on_focus_change(clone!(is_focused, move |new_focus| {
                is_focused.set(new_focus == FocusState::Focused)
            }))
            .build(),
        AnchoredContainerBuilder::new()
            .anchor_point(AnchorPoint::Center)
            .child_signal(clone!(
                label,
                clone!(lift_label, move || {
                    lift_label.signal().map(clone!(label, move |has_focus| {
                        if !has_focus {
                            LabelBuilder::new()
                                .font_settings(FontSettings {
                                    metrics: Metrics {
                                        font_size: 14.0,
                                        line_height: 14.0,
                                    },
                                    family: FamilyOwned::Monospace,
                                    stretch: Default::default(),
                                    style: Style::Italic,
                                    weight: Weight::THIN,
                                })
                                .text_color(label_color)
                                .text_signal(clone!(label, move || label
                                    .signal_cloned()
                                    .map(|v| v.into())))
                                .build()
                        } else {
                            StackBuilder::new().build()
                        }
                    }))
                })
            ))
            .build(),
        AnchoredContainerBuilder::new()
            .anchor_point(AnchorPoint::TopLeft)
            .padding(Padding {
                left: 2,
                right: 0,
                top: 1,
                bottom: 0,
            })
            .child_signal(clone!(
                value_bc,
                clone!(
                    label,
                    clone!(lift_label, move || lift_label.signal().map(clone!(
                        value_bc,
                        clone!(label, move |has_focus| {
                            if has_focus {
                                LabelBuilder::new()
                                    .font_settings(FontSettings {
                                        metrics: Metrics {
                                            font_size: 12.0,
                                            line_height: 10.0,
                                        },
                                        family: FamilyOwned::Monospace,
                                        stretch: Default::default(),
                                        style: Style::Italic,
                                        weight: Weight::THIN,
                                    })
                                    .text_color_signal(clone!(value_bc, move || value_bc
                                        .signal_cloned()
                                        .map(move |v| {
                                            if v.len() < 5 {
                                                label_color
                                            } else {
                                                label_color_invalid
                                            }
                                        })))
                                    .text_signal(clone!(label, move || label
                                        .signal_cloned()
                                        .map(|v| v.into())))
                                    .build()
                            } else {
                                StackBuilder::new().build()
                            }
                        })
                    )))
                )
            ))
            .build(),
        AnchoredContainerBuilder::new()
            .anchor_point(AnchorPoint::CenterLeft)
            .padding(Padding {
                left: 4,
                right: 0,
                top: 0,
                bottom: 0,
            })
            .child(
                LabelBuilder::new()
                    .font_settings(FontSettings {
                        metrics: Metrics {
                            font_size: 15.0,
                            line_height: 15.0,
                        },
                        family: FamilyOwned::Monospace,
                        stretch: Default::default(),
                        style: Style::Normal,
                        weight: Weight::NORMAL,
                    })
                    .text_color(theme.input.text_color)
                    .text_signal(clone!(value_bc, move || value_bc
                        .signal_cloned()
                        .map(|v| v.into())))
                    .build(),
            )
            .build(),
    ]);

    StackBuilder::new()
        .size_constraint(SizeConstraint::MaxHeight(40))
        .children_signal_vec(clone!(children, move || children.signal_vec_cloned()))
        .build()
}
