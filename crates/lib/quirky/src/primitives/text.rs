use glam::UVec2;
use glyphon::{Buffer, Color, FontSystem, Resolution, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer};
use wgpu::{Device, MultisampleState, Queue, RenderPass};
use crate::LayoutBox;
use crate::primitives::{DrawablePrimitive, RenderContext};

pub struct Text {
    text_renderer: TextRenderer,
}

impl Text {
    pub fn new(device: &Device, queue: &Queue, font_system: &mut FontSystem, atlas: &mut TextAtlas, cache: &mut SwashCache, multisample: MultisampleState, buffer: &Buffer, bb: LayoutBox, screen_resolution: UVec2) -> Self {
        let mut text_renderer = TextRenderer::new(atlas, device, multisample, None);

        text_renderer.prepare(
            device,
            queue,
            font_system,
            atlas,
            Resolution { width: screen_resolution.x, height: screen_resolution.y },
            [TextArea {
                buffer,
                left: bb.pos.x as f32,
                top: bb.pos.y as f32,
                scale: 1.0,
                bounds: TextBounds {
                    left: bb.pos.x as i32,
                    top: bb.pos.y as i32,
                    right: (bb.pos.x as i32 +bb.size.x as i32).min(screen_resolution.x as i32),
                    bottom: (bb.pos.y as i32 + bb.size.y as i32).min(screen_resolution.y as i32),
                },
                default_color: Color::rgb(80,80,50),
            }],
            cache,
        ).unwrap();

        Self {
            text_renderer,
        }
    }
}

impl DrawablePrimitive for Text {
    fn draw<'a>(&'a self, pass: &mut RenderPass<'a>, render_context: &RenderContext<'a>) {
        self.text_renderer.render(render_context.text_atlas, pass).unwrap();
    }
}