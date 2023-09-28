use futures_signals::signal::{Mutable, SignalExt};
use glam::UVec2;
use glyphon::{FamilyOwned, FontSystem, Metrics, SwashCache};
use lipsum::lipsum_words;
use quirky::styling::Padding;
use quirky::widget::Widget;
use quirky::{clone, MouseEvent, SizeConstraint, WidgetEvent};
use quirky_widgets::widgets::box_layout::{BoxLayoutBuilder, ChildDirection};
use quirky_widgets::widgets::button::ButtonBuilder;
use quirky_widgets::widgets::drawable_image::DrawableImageBuilder;
use quirky_widgets::widgets::label::{FontSettings, LabelBuilder};
use quirky_widgets::widgets::layout_item::LayoutItemBuilder;
use quirky_widgets::widgets::slab::SlabBuilder;
use quirky_widgets::widgets::stack::StackBuilder;
use quirky_widgets::widgets::text_input::{text_input, TextInputBuilder};
use quirky_widgets::widgets::text_layout::TextLayoutBuilder;
use quirky_winit::QuirkyWinitApp;
use rand::random;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    let boxed_layout = simple_panel_layout();

    let font_system = FontSystem::new();
    let font_cache = SwashCache::new();

    let (quirky_winit_app, quirky_app) = QuirkyWinitApp::new(boxed_layout, font_system, font_cache)
        .await
        .unwrap();

    let draw_notifier = quirky_winit_app.get_trigger_draw_callback();

    tokio::spawn(quirky_app.run(draw_notifier));
    quirky_winit_app.run_event_loop();
}

fn button_row(text: Mutable<String>) -> Arc<dyn Widget> {
    BoxLayoutBuilder::new()
        .size_constraint(SizeConstraint::MaxHeight(30))
        .child_direction(ChildDirection::Horizontal)
        .children(vec![
            ButtonBuilder::new()
                .on_click(clone!(text, move |_| text.set("Cat".to_string())))
                .content(LabelBuilder::new().text("🐈".into()).build())
                .build(),
            ButtonBuilder::new()
                .on_click(clone!(text, move |_| text.set("Dog".to_string())))
                .content(LabelBuilder::new().text("🐶".into()).build())
                .build(),
            ButtonBuilder::new()
                .on_click(clone!(text, move |_| text.set("Cow".to_string())))
                .content(LabelBuilder::new().text("C".into()).build())
                .build(),
            ButtonBuilder::new()
                .on_click(clone!(text, move |_| text.set("Donkey".to_string())))
                .content(LabelBuilder::new().text("D".into()).build())
                .build(),
        ])
        .build()
}

fn stack_panel(text: Mutable<String>) -> Arc<dyn Widget> {
    StackBuilder::new()
        .children(vec![
            SlabBuilder::new().color([0.01, 0.05, 0.01, 1.0]).build(),
            SlabBuilder::new()
                .on_event(clone!(text, move |e| {
                    match e.widget_event {
                        WidgetEvent::MouseEvent { event } => match event {
                            MouseEvent::ButtonDown { .. } => {
                                text.set(format!("{}!", text.get_cloned()));
                            }
                            _ => {}
                        },

                        _ => {}
                    }
                }))
                .color([0.01, 0.01, 0.1, 0.2])
                .build(),
        ])
        .build()
}

fn simple_panel_layout() -> Arc<dyn Widget> {
    let padding = Mutable::new(Padding::default());
    let text = Mutable::new("".to_string());

    tokio::spawn(clone!(padding, async move {
        loop {
            sleep(Duration::from_millis(500)).await;
            padding.set({
                Padding {
                    top: random::<u32>() % 100,
                    left: random::<u32>() % 100,
                    bottom: random::<u32>() % 200,
                    right: random::<u32>() % 200,
                }
            })
        }
    }));

    let children: Mutable<Vec<Arc<dyn Widget>>> = Mutable::new(vec![
        BoxLayoutBuilder::new()
            .children(vec![SlabBuilder::new().build()])
            .size_constraint(SizeConstraint::MaxHeight(150))
            .build(),
        BoxLayoutBuilder::new()
            .children(vec![
                BoxLayoutBuilder::new()
                    .child_direction(ChildDirection::Vertical)
                    .children(vec![
                        text_input(
                            text.signal_cloned(),
                            clone!(text, move |v| { text.set(v.to_string()) }),
                            |_| println!("submitted"),
                        ),
                        stack_panel(text.clone()),
                        LayoutItemBuilder::new()
                            .child(
                                LabelBuilder::new()
                                    .font_settings(FontSettings {
                                        family: FamilyOwned::Monospace,
                                        metrics: Metrics {
                                            font_size: 20.0,
                                            line_height: 16.0,
                                        },
                                        ..Default::default()
                                    })
                                    .text_signal(clone!(text, move || text
                                        .signal_cloned()
                                        .map(|t| t.into())))
                                    .build(),
                            )
                            .build(),
                        button_row(text.clone()),
                        ButtonBuilder::new()
                            .on_click(clone!(text, move |_| { text.set("".to_string()) }))
                            .content(
                                LabelBuilder::new()
                                    .font_settings(FontSettings {
                                        family: FamilyOwned::Monospace,
                                        metrics: Metrics {
                                            font_size: 15.0,
                                            line_height: 16.0,
                                        },
                                        ..Default::default()
                                    })
                                    .text("Clear 🌍".to_string().into())
                                    .build(),
                            )
                            .build(),
                        TextLayoutBuilder::new()
                            .text(lipsum_words(400).into())
                            .build(),
                        LayoutItemBuilder::new()
                            .child(
                                SlabBuilder::new()
                                    .size_constraint(SizeConstraint::MinSize(UVec2::new(100, 20)))
                                    .build(),
                            )
                            .build(),
                    ])
                    .size_constraint(SizeConstraint::MaxWidth(300))
                    .build(),
                BoxLayoutBuilder::new()
                    .child_direction(ChildDirection::Vertical)
                    .children(vec![DrawableImageBuilder::new().build()])
                    .size_constraint(SizeConstraint::Unconstrained)
                    .build(),
            ])
            .size_constraint(SizeConstraint::MinSize(UVec2::new(1, 150)))
            .child_direction(ChildDirection::Horizontal)
            .build(),
    ]);

    BoxLayoutBuilder::new()
        .children_signal(clone!(children, move || children.signal_cloned()))
        .child_direction(ChildDirection::Vertical)
        .build()
}
