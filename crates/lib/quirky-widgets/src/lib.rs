use crate::resources::font_resource::FontResource;
use crate::theming::QuirkyTheme;
use futures_signals::signal::Mutable;
use glyphon::{FontSystem, SwashCache, TextAtlas};
use quirky::quirky_app_context::{QuirkyAppContext, QuirkyResources};
use wgpu::TextureFormat;

pub mod components;
pub mod layouts;
pub mod primitives;
pub mod resources;
pub mod styling;
pub mod theming;
pub mod widgets;

pub fn init(
    resources: &mut QuirkyResources,
    context: &QuirkyAppContext,
    surface_format: TextureFormat,
    theme: Option<QuirkyTheme>,
) {
    resources.insert(FontResource {
        font_system: FontSystem::new(),
        font_cache: SwashCache::new(),
        text_atlas: TextAtlas::new(&context.device, &context.queue, surface_format),
    });

    let theme = theme.unwrap_or(QuirkyTheme::dark_default());

    resources.insert(Mutable::new(theme));
}
