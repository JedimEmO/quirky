use crate::primitives::{DrawablePrimitive, RenderContext};
use glyphon::TextRenderer;
use wgpu::RenderPass;

impl DrawablePrimitive for TextRenderer {
    fn draw<'a>(&'a self, pass: &mut RenderPass<'a>, render_context: &RenderContext<'a>) {
        self.render(render_context.text_atlas, pass);
    }
}
