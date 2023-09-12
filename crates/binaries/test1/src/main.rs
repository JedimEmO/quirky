use futures_signals::signal::Mutable;
use glam::UVec2;
use quirky::primitives::Quads;
use quirky::widget::Widget;
use quirky::widgets::box_layout::{BoxLayoutBuilder, ChildDirection};
use quirky::widgets::slab::SlabBuilder;
use quirky::{clone, MouseEvent, SizeConstraint, WidgetEvent};
use quirky_winit::QuirkyWinitApp;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let boxed_layout = simple_panel_layout();

    let (quirky_winit_app, quirky_app) = QuirkyWinitApp::new(boxed_layout).await.unwrap();

    quirky_app.configure_primitive::<Quads>();

    let draw_notifier = quirky_winit_app.get_trigger_draw_callback();

    tokio::spawn(quirky_app.run(draw_notifier));
    quirky_winit_app.run();
}

fn simple_panel_layout() -> Arc<dyn Widget> {
    let orientation = Mutable::new(ChildDirection::Vertical);

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
                        SlabBuilder::new()
                            .color([0.01, 0.01, 0.1, 1.0])
                            .on_event(clone!(orientation, move |e| {
                            match e.widget_event {
                                WidgetEvent::MouseEvent {event} => {
                                    match event {
                                        MouseEvent::ButtonDown { button: _ } => {
                                            if orientation.get() == ChildDirection::Horizontal {
                                                orientation.set(ChildDirection::Vertical);
                                            } else {
                                                orientation.set(ChildDirection::Horizontal);
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        })).build(),
                        SlabBuilder::new().build(),
                        SlabBuilder::new().build(),
                        SlabBuilder::new().build(),
                    ])
                    .size_constraint(SizeConstraint::MaxWidth(300))
                    .build(),
                BoxLayoutBuilder::new()
                    .child_direction(ChildDirection::Vertical)
                    .children(vec![SlabBuilder::new().build()])
                    .size_constraint(SizeConstraint::Unconstrained)
                    .build(),
            ])
            .size_constraint(SizeConstraint::MinSize(UVec2::new(1, 150)))
            .child_direction(ChildDirection::Horizontal)
            .build(),
    ]);

    BoxLayoutBuilder::new()
        .children_signal(clone!(children, move || children.signal_cloned()))
        .child_direction_signal(clone!(orientation, move || orientation.signal_cloned()))
        .build()
}
