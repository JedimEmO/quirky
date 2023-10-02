use crate::quirky_app_context::QuirkyResources;
use glam::UVec2;
use std::collections::HashMap;
use uuid::Uuid;
use wgpu::{BindGroup, BindGroupLayout, Device, Queue, RenderPipeline, TextureFormat};

pub struct RenderContext<'a> {
    pub resources: &'a mut QuirkyResources,
    pub camera_bind_group: &'a BindGroup,
    pub screen_resolution: UVec2,
    pub pipeline_cache: &'a HashMap<Uuid, RenderPipeline>,
    pub bind_group_cache: &'a HashMap<Uuid, BindGroup>,
}

pub struct PrepareContext<'a> {
    pub resources: &'a mut QuirkyResources,
    pub device: &'a Device,
    pub queue: &'a Queue,
    pub surface_format: TextureFormat,
    pub pipeline_cache: &'a mut HashMap<Uuid, RenderPipeline>,
    pub bind_group_cache: &'a mut HashMap<Uuid, BindGroup>,
    pub camera_bind_group_layout: &'a BindGroupLayout,
}
