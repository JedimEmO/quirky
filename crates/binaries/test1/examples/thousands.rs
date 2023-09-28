use glyphon::{FontSystem, SwashCache};
use quirky::widget::Widget;
use quirky_widgets::widgets::box_layout::{BoxLayoutBuilder, ChildDirection};
use quirky_widgets::widgets::slab::SlabBuilder;
use quirky_winit::QuirkyWinitApp;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let boxed_layout = thousands_layout();

    let font_system = FontSystem::new();
    let font_cache = SwashCache::new();

    let (quirky_winit_app, quirky_app) = QuirkyWinitApp::new(boxed_layout, font_system, font_cache)
        .await
        .unwrap();

    let draw_notifier = quirky_winit_app.get_trigger_draw_callback();

    tokio::spawn(quirky_app.run(draw_notifier));
    quirky_winit_app.run_event_loop();
}

fn thousands_layout() -> Arc<dyn Widget> {
    BoxLayoutBuilder::new()
        .child_direction(ChildDirection::Vertical)
        .children((0..100).map(|y| hundreds_row(y)).collect())
        .build()
}

fn hundreds_row(y: i32) -> Arc<dyn Widget> {
    BoxLayoutBuilder::new()
        .child_direction(ChildDirection::Horizontal)
        .children(
            (0..200)
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
        .build()
}
