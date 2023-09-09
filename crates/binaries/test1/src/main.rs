use futures_signals::signal::always;
use futures_signals::signal::Mutable;
use glam::UVec2;
use quirky::primitives::Quads;
use quirky::widget::widgets::Slab;
use quirky::widget::Widget;
use quirky::widgets::box_layout::{BoxLayout, ChildDirection};
use quirky::{clone, LayoutBox, SizeConstraint};
use quirky_winit::QuirkyWinitApp;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let children: Mutable<Vec<Arc<dyn Widget>>> = Mutable::new(vec![
        Arc::new(
            BoxLayout::builder()
                .children(|| always(vec![Arc::new(Slab::default()) as Arc<dyn Widget>]))
                .size_constraint(|| always(SizeConstraint::MaxHeight(150)))
                .child_direction(|| always(ChildDirection::Horizontal))
                .build(),
        ),
        Arc::new(
            BoxLayout::builder()
                .children(|| {
                    always(vec![
                        Arc::new(
                            BoxLayout::builder()
                                .child_direction(|| always(ChildDirection::Vertical))
                                .children(|| {
                                    always(vec![
                                        Arc::new(Slab::default()) as Arc<dyn Widget>,
                                        Arc::new(Slab::default()) as Arc<dyn Widget>,
                                        Arc::new(Slab::default()) as Arc<dyn Widget>,
                                        Arc::new(Slab::default()) as Arc<dyn Widget>,
                                    ])
                                })
                                .size_constraint(|| always(SizeConstraint::MaxWidth(300)))
                                .build(),
                        ) as Arc<dyn Widget>,
                        Arc::new(
                            BoxLayout::builder()
                                .child_direction(|| always(ChildDirection::Vertical))
                                .children(|| {
                                    always(vec![Arc::new(Slab::default()) as Arc<dyn Widget>])
                                })
                                .size_constraint(|| always(SizeConstraint::Unconstrained))
                                .build(),
                        ) as Arc<dyn Widget>,
                    ])
                })
                .size_constraint(|| always(SizeConstraint::MinSize(UVec2::new(1, 150))))
                .child_direction(|| always(ChildDirection::Horizontal))
                .build(),
        ),
    ]);

    let boxed_layout = Arc::new(
        BoxLayout::builder()
            .children(clone!(children, move || children.signal_cloned()))
            .child_direction(|| always(ChildDirection::Vertical))
            .size_constraint(|| always(SizeConstraint::Unconstrained))
            .bounding_box(Mutable::new(LayoutBox {
                pos: Default::default(),
                size: UVec2::new(800, 600),
            }))
            .build(),
    );

    let (quirky_winit_app, quirky_app) = QuirkyWinitApp::new(boxed_layout).await.unwrap();

    quirky_app.configure_primitive::<Quads>();

    let draw_notifier = quirky_winit_app.get_trigger_draw_callback();

    tokio::spawn(quirky_app.run(draw_notifier));
    quirky_winit_app.run();
}
