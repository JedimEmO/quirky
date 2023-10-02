use crate::resources::font_resource::FontResource;
use glyphon::{FontSystem, SwashCache, TextAtlas};
use quirky::QuirkyApp;
use wgpu::TextureFormat;

pub mod layouts;
pub mod primitives;
pub mod resources;
pub mod styling;
pub mod widgets;

pub fn init(quirky_app: &QuirkyApp, surface_format: TextureFormat) {
    quirky_app.resources.lock().unwrap().insert(FontResource {
        font_system: FontSystem::new(),
        font_cache: SwashCache::new(),
        text_atlas: TextAtlas::new(
            &quirky_app.context.device,
            &quirky_app.context.queue,
            surface_format,
        ),
    });
}
