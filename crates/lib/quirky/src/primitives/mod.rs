pub mod quad;
pub mod text;

use glam::UVec2;
use glyphon::TextAtlas;
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Device, TextureFormat};

pub struct RenderContext<'a> {
    pub text_atlas: &'a TextAtlas,
    pub camera_bind_group: &'a BindGroup,
    pub screen_resolution: UVec2
}

pub trait Primitive {
    fn configure_pipeline(
        device: &Device,
        bind_group_layouts: &[&BindGroupLayout],
        surface_format: TextureFormat,
    );
}

pub trait DrawablePrimitive {
    fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, ctx: &RenderContext<'a>);
}
