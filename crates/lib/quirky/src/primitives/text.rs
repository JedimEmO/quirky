use crate::primitives::{DrawablePrimitive, RenderContext};
use crate::quirky_app_context::FontResource;
use glyphon::TextRenderer;
use std::any::TypeId;
use wgpu::RenderPass;

impl DrawablePrimitive for TextRenderer {
    fn draw<'a>(&'a self, pass: &mut RenderPass<'a>, render_context: &'a RenderContext<'a>) {
        let font_resource: &FontResource = render_context
            .resources
            .get_resource(TypeId::of::<FontResource>())
            .unwrap();

        let _ = self.render(&font_resource.text_atlas, pass);
    }
}
