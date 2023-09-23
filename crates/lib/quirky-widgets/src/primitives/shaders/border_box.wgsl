struct UiCameraUniform {
    transform: mat4x4<f32>
}

@group(0) @binding(0)
var<uniform> camera: UiCameraUniform;

struct VertexInput {
    @location(0) pos: vec2<f32>
};

struct BorderBoxData {
    @location(2) pos: vec2<f32>,
    @location(3) size: vec2<f32>,
    @location(4) color: vec4<f32>,
    @location(5) shade_color: vec4<f32>,
    @location(6) border_side: u32,
    @location(7) borders: vec4<u32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(linear) quad_pos: vec2<f32>,
    @location(3) shade_color: vec4<f32>,
    @location(4) border_side: u32,
    @location(5) borders: vec4<u32>,
    @location(6) box_size: vec2<f32>
};

@vertex
fn vs_main(vert: VertexInput, q: BorderBoxData) -> VertexOutput {
    var out: VertexOutput;

    let box_x = vert.pos.x * q.size.x;
    let box_y = vert.pos.y * q.size.y;
    let x = box_x + q.pos.x;
    let y = box_y + q.pos.y;

    out.position = camera.transform * vec4<f32>(x, y, 0.0, 1.0);
    out.color = q.color;
    out.quad_pos = vec2<f32>(box_x, box_y);
    out.shade_color = q.shade_color;
    out.border_side = q.border_side;
    out.borders = q.borders;
    out.box_size = q.size;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let border_thickness = 1.0;
    let centered = in.quad_pos - in.box_size * 0.5;
    let in_border = (abs(centered.x) + border_thickness) - in.box_size.x / 2.0 > 0.0 || (abs(centered.y) + border_thickness) - in.box_size.y / 2.0 > 0.0;
    let alpha = select(0.0, 1.0, in_border);

    return vec4<f32>(in.color.x, in.color.y, in.color.z, alpha);
}