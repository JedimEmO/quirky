use crate::primitives::image::ImagePrimitive;
use crate::primitives::{DrawablePrimitive, PrepareContext};
use crate::quirky_app_context::QuirkyAppContext;
use crate::widget::{Widget, WidgetBase};
use crate::{clone, LayoutBox, MouseButton, MouseEvent, WidgetEvent};
use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use futures_signals::signal::{Mutable, SignalExt};
use glam::UVec2;
use image::{Rgba, RgbaImage};
use quirky_macros::widget;
use std::sync::Arc;
use uuid::Uuid;

#[widget]
pub struct DrawableImage {
    #[default(Mutable::new(image::RgbaImage::new(64, 64)))]
    image: Mutable<RgbaImage>,
}

#[async_trait]
impl Widget for DrawableImage {
    fn paint(
        &self,
        _ctx: &QuirkyAppContext,
        _paint_ctx: &mut PrepareContext,
    ) -> Vec<Box<dyn DrawablePrimitive>> {
        let bb = self.bounding_box.get();

        let mut image = self.image.get_cloned();

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

        let event_redraw = widget_events.for_each(clone!(self, move |e| {
            clone!(
                mouse_pos,
                clone!(self, async move {
                    let ec = e.clone();

                    match e {
                        WidgetEvent::MouseEvent { event } => match event {
                            MouseEvent::ButtonDown { button } => {
                                if button == MouseButton::Middle {
                                    self.image.lock_mut().fill(0)
                                }
                            }
                            MouseEvent::Move { pos } => mouse_pos.set(pos),
                            MouseEvent::Drag { from, to, button } => {
                                if button != MouseButton::Middle {
                                    let mut image = self.image.lock_mut();
                                    let bb = self.bounding_box.get();

                                    let to = mouse_pos.get();
                                    let put_x = to.x as i32 - bb.pos.x as i32;
                                    let put_y = to.y as i32 - bb.pos.y as i32;

                                    let px = (put_x as f32 / bb.size.x as f32) * 64.0;
                                    let py = (put_y as f32 / bb.size.y as f32) * 64.0;

                                    let color = match button {
                                        MouseButton::Left => Rgba([255, 0, 0, 255]),
                                        _ => Rgba([0, 255, 0, 255]),
                                    };

                                    if px >= 0.0 && px < 64.0 && py >= 0.0 && py < 64.0 {
                                        image.put_pixel(px as u32, py as u32, color);
                                    }
                                }

                                self.set_dirty();
                                ctx.signal_redraw().await;
                            }
                            _ => {}
                        },
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
