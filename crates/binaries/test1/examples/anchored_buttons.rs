use glam::UVec2;
use glyphon::{FamilyOwned, Metrics};
use quirky::widget::SizeConstraint;
use quirky::widget::Widget;
use quirky_widgets::layouts::anchored_container::{AnchorPoint, AnchoredContainerBuilder};
use quirky_widgets::widgets::button::ButtonBuilder;
use quirky_widgets::widgets::label::{FontSettings, LabelBuilder};
use quirky_widgets::widgets::stack::StackBuilder;
use quirky_winit::QuirkyWinitApp;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let layout = StackBuilder::new()
        .children(vec![
            anchored(button("Top Left"), AnchorPoint::TopLeft),
            anchored(button("Top Center"), AnchorPoint::TopCenter),
            anchored(button("Top Right"), AnchorPoint::TopRight),
            anchored(button("Center Left"), AnchorPoint::CenterLeft),
            anchored(button("Center"), AnchorPoint::Center),
            anchored(button("Center Right"), AnchorPoint::CenterRight),
            anchored(button("Bottom Left"), AnchorPoint::BottomLeft),
            anchored(button("Bottom Center"), AnchorPoint::BottomCenter),
            anchored(button("Bottom Right"), AnchorPoint::BottomRight),
        ])
        .build();

    let (quirky_winit_app, quirky_app) = QuirkyWinitApp::new(layout).await.unwrap();
    quirky_widgets::init(&quirky_app, quirky_winit_app.surface_format);
    let draw_notifier = quirky_winit_app.get_trigger_draw_callback();

    tokio::spawn(quirky_app.run(draw_notifier));
    quirky_winit_app.run_event_loop();
}

fn button(text: impl ToString) -> Arc<dyn Widget> {
    ButtonBuilder::new()
        .size_constraint(SizeConstraint::MaxSize(UVec2::new(200, 100)))
        .on_click(|_| {})
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
                .text(text.to_string().into())
                .build(),
        )
        .build()
}

fn anchored(widget: Arc<dyn Widget>, point: AnchorPoint) -> Arc<dyn Widget> {
    AnchoredContainerBuilder::new()
        .anchor_point(point)
        .child(widget)
        .build()
}
