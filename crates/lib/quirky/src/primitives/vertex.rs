use wgpu_macros::VertexLayout;

#[derive(VertexLayout, bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
#[repr(C)]
pub struct Vertex {
    pos: [f32; 2],
}

pub const VERTICES: [Vertex; 4] = [
    Vertex { pos: [0.0, 0.0] },
    Vertex { pos: [1.0, 0.0] },
    Vertex { pos: [1.0, 1.0] },
    Vertex { pos: [0.0, 1.0] },
];
