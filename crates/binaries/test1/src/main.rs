use futures::stream::StreamExt;
use futures_signals::signal::{Mutable, SignalExt};
use glam::UVec2;
use glyphon::cosmic_text::Align;
use glyphon::{FamilyOwned, Metrics};
use inotify::WatchMask;
use lipsum::lipsum_words;
use quirky::clone;
use quirky::quirky_app_context::{QuirkyAppContext, QuirkyResources};
use quirky::widget::{SizeConstraint, Widget};
use quirky::widgets::events::{MouseEvent, WidgetEvent};
use quirky_widgets::components::text_input::text_input;
use quirky_widgets::layouts::anchored_container::AnchoredContainerBuilder;
use quirky_widgets::layouts::box_layout::{BoxLayoutBuilder, ChildDirection};
use quirky_widgets::theming::QuirkyTheme;
use quirky_widgets::widgets::button::ButtonBuilder;
use quirky_widgets::widgets::drawable_image::DrawableImageBuilder;
use quirky_widgets::widgets::label::{FontSettings, LabelBuilder};
use quirky_widgets::widgets::slab::SlabBuilder;
use quirky_widgets::widgets::stack::StackBuilder;
use quirky_widgets::widgets::text_layout::TextLayoutBuilder;
use quirky_winit::QuirkyWinitApp;
use std::sync::{Arc, Mutex};
use std::{env, fs};
use wgpu::TextureFormat;

#[tokio::main]
async fn main() {
    let (quirky_winit_app, quirky_app) = QuirkyWinitApp::new(
        |resources: &mut QuirkyResources,
         context: &QuirkyAppContext,
         surface_format: TextureFormat| {
            quirky_widgets::init(
                resources,
                context,
                surface_format,
                Some(QuirkyTheme::dark_default()),
            )
        },
        simple_panel_layout,
    )
    .await
    .unwrap();

    let draw_notifier = quirky_winit_app.get_trigger_draw_callback();

    tokio::spawn(quirky_app.run(draw_notifier));
    quirky_winit_app.run_event_loop();
}

fn button_row(text: Mutable<String>, theme: Mutable<QuirkyTheme>) -> Arc<dyn Widget> {
    BoxLayoutBuilder::new()
        .size_constraint(SizeConstraint::MaxHeight(30))
        .child_direction(ChildDirection::Horizontal)
        .children(vec![
            ButtonBuilder::new()
                .on_click(clone!(text, move |_| text.set("Cat".to_string())))
                .content(
                    LabelBuilder::new()
                        .text_align(Align::Center)
                        .text("🐈".into())
                        .build(),
                )
                .build(),
            ButtonBuilder::new()
                .on_click(clone!(text, move |_| text.set("Dog".to_string())))
                .content(
                    LabelBuilder::new()
                        .text_align(Align::Center)
                        .text("🐶".into())
                        .build(),
                )
                .build(),
            ButtonBuilder::new()
                .on_click(clone!(theme, move |_| theme.set(QuirkyTheme::dark_default())))
                .content(
                    LabelBuilder::new()
                        .text_align(Align::Center)
                        .text("Dark".into())
                        .build(),
                )
                .build(),
            ButtonBuilder::new()
                .on_click(clone!(theme, move |_| theme.set(QuirkyTheme::light_default())))
                .content(
                    LabelBuilder::new()
                        .text_align(Align::Center)
                        .text("Light".into())
                        .build(),
                )
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
                    if let WidgetEvent::MouseEvent {
                        event: MouseEvent::ButtonDown { .. },
                    } = e.widget_event
                    {
                        text.set(format!("{}!", text.get_cloned()));
                    }
                }))
                .color([0.01, 0.01, 0.1, 0.2])
                .build(),
        ])
        .build()
}

fn simple_panel_layout(resources: Arc<Mutex<QuirkyResources>>) -> Arc<dyn Widget> {
    let text = Mutable::new("".to_string());

    let mutable_theme = resources
        .lock()
        .unwrap()
        .get_resource::<Mutable<QuirkyTheme>>()
        .unwrap()
        .clone();

    tokio::spawn(clone!(mutable_theme, async move {
        let inot = inotify::Inotify::init().unwrap();
        let current_dir = env::current_dir()
            .expect("failed to get current dir")
            .join("theme.toml");
        inot.watches()
            .add(current_dir, WatchMask::MODIFY)
            .expect("failed to add watch");

        let buffer = [0u8; 4098];

        inot.into_event_stream(buffer)
            .unwrap()
            .for_each(move |_event| {
                clone!(mutable_theme, async move {
                    let current_dir = env::current_dir()
                        .expect("failed to get current dir")
                        .join("theme.toml");
                    let theme = fs::read_to_string(current_dir.as_path()).unwrap();
                    let theme: QuirkyTheme = toml::from_str(&theme).unwrap();
                    mutable_theme.set(theme);
                })
            })
            .await;
    }));

    BoxLayoutBuilder::new()
        .children_signal_vec(clone!(text, move || {
            mutable_theme
                .signal_ref(clone!(
                    mutable_theme,
                    clone!(text, move |theme| {
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
                                                clone!(text, move |v| { text.set(v) }),
                                                |_| println!("submitted"),
                                                &theme,
                                            ),
                                            stack_panel(text.clone()),
                                            AnchoredContainerBuilder::new()
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
                                            button_row(text.clone(), mutable_theme.clone()),
                                            ButtonBuilder::new()
                                                .on_click(clone!(text, move |_| {
                                                    text.set("".to_string())
                                                }))
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
                                                        .text_align(Align::Center)
                                                        .text("Clear 🌍".to_string().into())
                                                        .build(),
                                                )
                                                .build(),
                                            TextLayoutBuilder::new()
                                                .text(lipsum_words(400).into())
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

                        vec![BoxLayoutBuilder::new()
                            .children_signal_vec(clone!(children, move || children
                                .signal_cloned()
                                .to_signal_vec()))
                            .child_direction(ChildDirection::Vertical)
                            .build()]
                    })
                ))
                .to_signal_vec()
        }))
        .build()
}
