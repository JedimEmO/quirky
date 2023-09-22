struct UiCameraUniform {
    transform: mat4x4<f32>
}

@group(0) @binding(0)
var<uniform> camera: UiCameraUniform;

struct VertexInput {
    @location(0) pos: vec2<f32>
};

struct ButtonData {
    @location(2) pos: vec2<f32>,
    @location(3) size: vec2<f32>,
    @location(4) color: vec4<f32>
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(linear) quad_pos: vec2<f32>
};

@vertex
fn vs_main(vert: VertexInput, q: ButtonData) -> VertexOutput {
    var out: VertexOutput;

    let x = vert.pos.x * q.size.x + q.pos.x;
    let y = vert.pos.y * q.size.y + q.pos.y;

    out.position = camera.transform * vec4<f32>(x, y, 0.0, 1.0);
    out.color = q.color;
    out.quad_pos = vert.pos;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let centered = in.quad_pos - vec2<f32>(0.5, 0.5);
    let distance = max(length(centered), 0.3);
    let factor = distance;

    let r = in.color.x;
    let g = in.color.y;
    let b = in.color.z;

    return vec4<f32>(r * factor, g* factor, b * factor, 1.0);
}