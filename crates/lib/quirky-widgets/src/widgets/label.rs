use async_trait::async_trait;
use futures::FutureExt;
use futures_signals::signal::{Signal, SignalExt};
use glam::UVec2;
use glyphon::{
    Attrs, Buffer, Color, FamilyOwned, Metrics, Resolution, Shaping, Stretch, Style, TextArea,
    TextBounds, TextRenderer, Weight,
};
use quirky::primitives::{DrawablePrimitive, PrepareContext};
use quirky::quirky_app_context::QuirkyAppContext;
use quirky::widget::{default_run, Widget};
use quirky::SizeConstraint;
use quirky_macros::widget;
use std::borrow::BorrowMut;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct FontSettings {
    pub metrics: Metrics,
    pub family: FamilyOwned,
    pub stretch: Stretch,
    pub style: Style,
    pub weight: Weight,
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            metrics: Metrics {
                font_size: 12.0,
                line_height: 15.0,
            },
            family: FamilyOwned::Serif,
            stretch: Default::default(),
            style: Default::default(),
            weight: Default::default(),
        }
    }
}

#[widget]
pub struct Label {
    #[signal_prop]
    text: Arc<str>,
    #[signal_prop]
    #[default(Default::default())]
    font_settings: FontSettings,
    #[default(Arc::new(RwLock::new(None)))]
    text_buffer: Arc<RwLock<Option<Buffer>>>,
}

#[async_trait]
impl<
        TextSignal: futures_signals::signal::Signal<Item = Arc<str>> + Send + Sync + Unpin + 'static,
        TextSignalFn: Fn() -> TextSignal + Send + Sync + 'static,
        FontSettingsSignal: futures_signals::signal::Signal<Item = FontSettings> + Send + Sync + Unpin + 'static,
        FontSettingsSignalFn: Fn() -> FontSettingsSignal + Send + Sync + 'static,
    > Widget for Label<TextSignal, TextSignalFn, FontSettingsSignal, FontSettingsSignalFn>
{
    fn paint(
        &self,
        quirky_context: &QuirkyAppContext,
        paint_quirky_context: &mut PrepareContext,
    ) -> Vec<Box<dyn DrawablePrimitive>> {
        let bb = self.bounding_box.get();
        let mut buffer_lock = self.text_buffer.write().unwrap();

        let font_settings = self.font_settings_prop_value.get_cloned().unwrap();

        let buffer = if let Some(mut buf) = buffer_lock.take() {
            buf.set_size(
                &mut paint_quirky_context.font_system,
                bb.size.x as f32,
                bb.size.y as f32,
            );

            buf.set_text(
                &mut paint_quirky_context.font_system,
                &self.text_prop_value.get_cloned().unwrap(),
                Attrs::new().family(font_settings.family.as_family()),
                Shaping::Advanced,
            );
            buf.shape_until_scroll(&mut paint_quirky_context.font_system);
            buf
        } else {
            let mut buffer = Buffer::new(paint_quirky_context.font_system, font_settings.metrics);

            buffer.set_size(
                &mut paint_quirky_context.font_system,
                bb.size.x as f32,
                bb.size.y as f32,
            );

            buffer.set_text(
                &mut paint_quirky_context.font_system,
                &self.text_prop_value.get_cloned().unwrap(),
                Attrs::new().family(font_settings.family.as_family()),
                Shaping::Advanced,
            );

            buffer.shape_until_scroll(&mut paint_quirky_context.font_system);

            buffer
        };

        let mut renderer = TextRenderer::new(
            paint_quirky_context.text_atlas.borrow_mut(),
            &quirky_context.device,
            Default::default(),
            None,
        );

        let screen_resolution = quirky_context.viewport_size.get();
        let buffer = buffer_lock.insert(buffer);

        let _ = renderer.prepare(
            &quirky_context.device,
            &quirky_context.queue,
            paint_quirky_context.font_system.borrow_mut(),
            paint_quirky_context.text_atlas.borrow_mut(),
            Resolution {
                width: screen_resolution.x,
                height: screen_resolution.y,
            },
            [TextArea {
                buffer,
                left: bb.pos.x as f32,
                top: bb.pos.y as f32,
                scale: 1.0,
                bounds: TextBounds {
                    left: bb.pos.x as i32,
                    top: bb.pos.y as i32,
                    right: (bb.pos.x as i32 + bb.size.x as i32).min(screen_resolution.x as i32),
                    bottom: (bb.pos.y as i32 + bb.size.y as i32).min(screen_resolution.y as i32),
                },
                default_color: Color::rgb(80, 80, 50),
            }],
            paint_quirky_context.font_cache.borrow_mut(),
        );

        vec![Box::new(renderer)]
    }

    fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send> {
        let font_settings_prop = self.font_settings_prop_value.clone();

        Box::new((self.text)().map(move |txt| {
            let font_settings = font_settings_prop.lock_ref();

            let metrics = font_settings.as_ref().unwrap().metrics;
            let len = txt.len();

            SizeConstraint::MinSize(UVec2::new(
                (metrics.font_size * len as f32 / 1.8) as u32,
                metrics.line_height as u32,
            ))
        }))
    }

    async fn run(self: Arc<Self>, quirky_context: &QuirkyAppContext) {
        default_run(self, quirky_context).await
    }
}