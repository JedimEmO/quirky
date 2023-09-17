use crate::primitives::image::ImagePrimitive;
use crate::primitives::quad::{Quad, Quads};
use crate::primitives::{DrawablePrimitive, PrepareContext};
use crate::quirky_app_context::QuirkyAppContext;
use crate::widget::{Event, Widget, WidgetBase};
use crate::{clone, LayoutBox, MouseEvent, SizeConstraint, WidgetEvent};
use async_std::task::sleep;
use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use futures_signals::signal::{always, Signal};
use futures_signals::signal::{Mutable, SignalExt};
use glam::{uvec2, UVec2};
use image::{Rgba, RgbaImage};
use quirky_macros::widget;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
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

        let event_redraw = widget_events.for_each(clone!(self, move |e| {
            clone!(self, async move {
                let ec = e.clone();

                match e {
                    WidgetEvent::MouseEvent { event } => match event {
                        MouseEvent::Drag { from, to } => {
                            {
                                let mut image = self.image.lock_mut();
                                let bb = self.bounding_box.get();

                                let put_x = to.x as i32 - bb.pos.x as i32;
                                let put_y = to.y as i32 - bb.pos.y as i32;

                                let px = (put_x as f32 / bb.size.x as f32) * 64.0;
                                let py = (put_y as f32 / bb.size.y as f32) * 64.0;

                                if px >= 0.0 && px < 64.0 && py >= 0.0 && py < 64.0 {
                                    image.put_pixel(px as u32, py as u32, Rgba([255, 0, 0, 255]));
                                }
                            }
                            self.set_dirty();
                            ctx.signal_redraw().await;
                        }
                        _ => {}
                    },
                }
            })
        }));

        let mut futs = self.poll_prop_futures(ctx);

        futs.push(event_redraw.boxed());

        loop {
            let _n = futs.select_next_some().await;
        }
    }
}
