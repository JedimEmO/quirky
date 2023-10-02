use crate::render_contexts::{PrepareContext, RenderContext};

pub trait DrawablePrimitive: Send + Sync {
    fn prepare(&mut self, _prepare_context: &mut PrepareContext) {}
    fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, render_context: &'a RenderContext<'a>);
}
