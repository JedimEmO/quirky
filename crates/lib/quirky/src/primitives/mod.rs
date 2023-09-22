pub mod image;
pub mod quad;
pub mod text;
pub mod vertex;

use glam::UVec2;
use glyphon::{FontSystem, SwashCache, TextAtlas};
use std::collections::HashMap;
use uuid::Uuid;
use wgpu::{BindGroup, BindGroupLayout, Device, Queue, RenderPipeline, TextureFormat};

pub struct RenderContext<'a> {
    pub text_atlas: &'a TextAtlas,
    pub camera_bind_group: &'a BindGroup,
    pub screen_resolution: UVec2,
    pub pipeline_cache: &'a HashMap<Uuid, RenderPipeline>,
    pub bind_group_cache: &'a HashMap<Uuid, BindGroup>,
}

pub struct PrepareContext<'a> {
    pub font_system: &'a mut FontSystem,
    pub text_atlas: &'a mut TextAtlas,
    pub font_cache: &'a mut SwashCache,
    pub device: &'a Device,
    pub queue: &'a Queue,
    pub surface_format: TextureFormat,
    pub pipeline_cache: &'a mut HashMap<Uuid, RenderPipeline>,
    pub bind_group_cache: &'a mut HashMap<Uuid, BindGroup>,
    pub camera_bind_group_layout: &'a BindGroupLayout,
}

pub trait Primitive {
    fn configure_pipeline(
        device: &Device,
        bind_group_layouts: &[&BindGroupLayout],
        surface_format: TextureFormat,
    );
}

pub trait DrawablePrimitive: Send + Sync {
    fn prepare(&mut self, _render_context: &mut PrepareContext) -> () {}
    fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, ctx: &RenderContext<'a>);
}
