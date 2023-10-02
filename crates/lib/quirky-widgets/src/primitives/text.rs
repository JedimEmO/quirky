use crate::resources::font_resource::FontResource;
use glyphon::TextRenderer;
use quirky::drawable_primitive::DrawablePrimitive;
use quirky::render_contexts::RenderContext;
use std::any::TypeId;
use wgpu::RenderPass;

pub struct TextRendererPrimitive(pub TextRenderer);

impl DrawablePrimitive for TextRendererPrimitive {
    fn draw<'a>(&'a self, pass: &mut RenderPass<'a>, render_context: &'a RenderContext<'a>) {
        let font_resource: &FontResource = render_context
            .resources
            .get_resource(TypeId::of::<FontResource>())
            .unwrap();

        let _ = self.0.render(&font_resource.text_atlas, pass);
    }
}
