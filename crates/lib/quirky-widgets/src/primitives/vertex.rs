use wgpu_macros::VertexLayout;

#[derive(VertexLayout, bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
#[repr(C)]
pub struct Vertex {
    pos: [f32; 2],
    tex_coords: [f32; 2],
}

pub const QUAD_VERTICES: [Vertex; 4] = [
    Vertex {
        pos: [0.0, 0.0],
        tex_coords: [0.0, 0.0],
    },
    Vertex {
        pos: [1.0, 0.0],
        tex_coords: [1.0, 0.0],
    },
    Vertex {
        pos: [1.0, 1.0],
        tex_coords: [1.0, 1.0],
    },
    Vertex {
        pos: [0.0, 1.0],
        tex_coords: [0.0, 1.0],
    },
];

pub const QUAD_INDEXES: [u16; 6] = [0, 1, 2, 0, 2, 3];
