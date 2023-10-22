use quirky::quirky_app_context::{QuirkyAppContext, QuirkyResources};
use quirky::widget::Widget;
use quirky_widgets::layouts::box_layout::{BoxLayoutBuilder, ChildDirection};
use quirky_widgets::theming::QuirkyTheme;
use quirky_widgets::widgets::slab::SlabBuilder;
use quirky_winit::QuirkyWinitApp;
use std::sync::Arc;
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
        |_| thousands_layout(),
    )
    .await
    .unwrap();

    let draw_notifier = quirky_winit_app.get_trigger_draw_callback();

    tokio::spawn(quirky_app.run(draw_notifier));
    quirky_winit_app.run_event_loop();
}

fn thousands_layout() -> Arc<dyn Widget> {
    BoxLayoutBuilder::new()
        .child_direction(ChildDirection::Vertical)
        .children(vec![
            BoxLayoutBuilder::new()
                .child_direction(ChildDirection::Vertical)
                .children((0..100).map(hundreds_row).collect())
                .build(),
            BoxLayoutBuilder::new()
                .child_direction(ChildDirection::Vertical)
                .children((0..100).map(hundreds_row).collect())
                .build(),
        ])
        .build()
}

fn hundreds_row(y: i32) -> Arc<dyn Widget> {
    BoxLayoutBuilder::new()
        .child_direction(ChildDirection::Horizontal)
        .children(vec![
            BoxLayoutBuilder::new()
                .child_direction(ChildDirection::Horizontal)
                .children(
                    (0..50)
                        .map(|x| {
                            SlabBuilder::new()
                                .color(if (x + y) % 2 == 0 {
                                    [0.0, 0.1, 0.0, 1.0]
                                } else {
                                    [0.1, 0.0, 0.0, 1.0]
                                })
                                .build()
                        })
                        .collect(),
                )
                .build(),
            BoxLayoutBuilder::new()
                .child_direction(ChildDirection::Horizontal)
                .children(
                    (0..50)
                        .map(|x| {
                            SlabBuilder::new()
                                .color(if (x + y) % 2 == 0 {
                                    [0.0, 0.1, 0.0, 1.0]
                                } else {
                                    [0.1, 0.0, 0.0, 1.0]
                                })
                                .build()
                        })
                        .collect(),
                )
                .build(),
            BoxLayoutBuilder::new()
                .child_direction(ChildDirection::Horizontal)
                .children(
                    (0..50)
                        .map(|x| {
                            SlabBuilder::new()
                                .color(if (x + y) % 2 == 0 {
                                    [0.0, 0.1, 0.0, 1.0]
                                } else {
                                    [0.1, 0.0, 0.0, 1.0]
                                })
                                .build()
                        })
                        .collect(),
                )
                .build(),
            BoxLayoutBuilder::new()
                .child_direction(ChildDirection::Horizontal)
                .children(
                    (0..50)
                        .map(|x| {
                            SlabBuilder::new()
                                .color(if (x + y) % 2 == 0 {
                                    [0.0, 0.1, 0.0, 1.0]
                                } else {
                                    [0.1, 0.0, 0.0, 1.0]
                                })
                                .build()
                        })
                        .collect(),
                )
                .build(),
        ])
        .build()
}
