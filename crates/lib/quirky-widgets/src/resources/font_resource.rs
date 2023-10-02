use glyphon::{FontSystem, SwashCache, TextAtlas};

pub struct FontResource {
    pub font_system: FontSystem,
    pub font_cache: SwashCache,
    pub text_atlas: TextAtlas,
}
