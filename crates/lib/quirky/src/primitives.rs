use glam::UVec2;
use wgpu::util::DeviceExt;

#[derive(bytemuck::Zeroable, bytemuck::Pod, Copy, Clone)]
#[repr(C)]
pub struct Vertex {
    pos: [f32; 2],
}

impl Vertex {
    pub fn buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
            }],
        }
    }
}

pub struct Quad {
    pub vertices: wgpu::Buffer,
    pub indexes: wgpu::Buffer,
}

const INDEXES: [u16; 6] = [0, 1, 2, 0, 2, 3];

impl Quad {
    pub fn new(device: &wgpu::Device, pos: UVec2, size: UVec2) -> Self {
        let left_x = pos.x as f32;
        let right_x = (pos.x + size.x) as f32;
        let top_y = pos.y as f32;
        let bottom_y = (pos.y + size.y) as f32;

        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad buffer"),
            contents: bytemuck::cast_slice(&[
                Vertex {
                    pos: [left_x, top_y],
                },
                Vertex {
                    pos: [right_x, top_y],
                },
                Vertex {
                    pos: [right_x, bottom_y],
                },
                Vertex {
                    pos: [left_x, bottom_y],
                },
            ]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let indexes = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad index buffer"),
            contents: bytemuck::cast_slice(&INDEXES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self { vertices, indexes }
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        // pass.set_scissor_rect(20, 20, 100, 20);
        pass.set_index_buffer(self.indexes.slice(..), wgpu::IndexFormat::Uint16);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw_indexed(0..6, 0, 0..1);
    }
}
