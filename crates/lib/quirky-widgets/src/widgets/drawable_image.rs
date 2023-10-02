use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use futures_signals::signal::{Mutable, SignalExt};
use glam::UVec2;
use image::{Rgba, RgbaImage};
use quirky::primitives::image::ImagePrimitive;
use quirky::primitives::{DrawablePrimitive, PrepareContext};
use quirky::quirky_app_context::QuirkyAppContext;
use quirky::widget::{Widget, WidgetBase};
use quirky::{clone, MouseButton, MouseEvent, WidgetEvent};
use quirky_macros::widget;
use std::sync::Arc;
use uuid::Uuid;

#[widget]
pub struct DrawableImage {
    #[default(Mutable::new(image::RgbaImage::new(1024, 1024)))]
    image: Mutable<RgbaImage>,
}

#[async_trait]
impl Widget for DrawableImage {
    fn prepare(
        &self,
        _ctx: &QuirkyAppContext,
        _paint_ctx: &mut PrepareContext,
    ) -> Vec<Box<dyn DrawablePrimitive>> {
        let bb = self.bounding_box.get();

        let image = self.image.get_cloned();

        let tex = ImagePrimitive {
            data: image,
            vertex_buffer: None,
            index_buffer: None,
            instance_buffer: None,
            bb,
        };

        vec![Box::new(tex)]
    }

    fn get_widget_at(&self, pos: UVec2, mut path: Vec<Uuid>) -> Option<Vec<Uuid>> {
        let bb = self.bounding_box.get();

        if bb.contains(pos) {
            path.push(self.id());

            Some(path)
        } else {
            None
        }
    }

    async fn run(self: Arc<Self>, ctx: &QuirkyAppContext) {
        let widget_events = ctx.subscribe_to_widget_events(self.id());
        let mouse_pos = Mutable::new(UVec2::default());

        let event_redraw = widget_events.await.for_each(clone!(self, move |e| {
            clone!(
                mouse_pos,
                clone!(self, async move {
                    match e {
                        WidgetEvent::MouseEvent { event } => match event {
                            MouseEvent::ButtonDown { button } => {
                                if button == MouseButton::Middle {
                                    self.image.lock_mut().fill(0)
                                }
                            }
                            MouseEvent::Move { pos } => mouse_pos.set(pos),
                            MouseEvent::Drag { button, .. } => {
                                if button != MouseButton::Middle {
                                    let mut image = self.image.lock_mut();
                                    let bb = self.bounding_box.get();

                                    let to = mouse_pos.get();
                                    let put_x = to.x as i32 - bb.pos.x as i32;
                                    let put_y = to.y as i32 - bb.pos.y as i32;

                                    let px = (put_x as f32 / bb.size.x as f32) * 1024.0;
                                    let py = (put_y as f32 / bb.size.y as f32) * 1024.0;

                                    let color = match button {
                                        MouseButton::Left => Rgba([255, 0, 0, 255]),
                                        _ => Rgba([0, 255, 0, 255]),
                                    };

                                    if (0.0..1024.0).contains(&px) && (0.0..1024.0).contains(&py) {
                                        image.put_pixel(px as u32, py as u32, color);
                                    }
                                }

                                self.set_dirty();
                                ctx.signal_redraw().await;
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                })
            )
        }));

        let mut futs = self.poll_prop_futures(ctx);

        futs.push(event_redraw.boxed());

        loop {
            let _n = futs.select_next_some().await;
        }
    }
}
