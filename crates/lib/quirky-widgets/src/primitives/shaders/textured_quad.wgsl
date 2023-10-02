struct UiCameraUniform {
    transform: mat4x4<f32>
}

@group(0) @binding(0)
var<uniform> camera: UiCameraUniform;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1)@binding(1)
var s_diffuse: sampler;

struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) tex_coords: vec2<f32>
};

struct QuadInfo {
    @location(3) pos: vec2<f32>,
    @location(4) size: vec2<f32>,
    @location(5) color: vec4<f32>
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>
};

@vertex
fn vs_main(vert: VertexInput, q: QuadInfo) -> VertexOutput {
    var out: VertexOutput;

    let x = vert.pos.x * q.size.x + q.pos.x;
    let y = vert.pos.y * q.size.y + q.pos.y;

    out.position = camera.transform * vec4<f32>(x, y, 0.0, 1.0);
    out.color = q.color;
    out.tex_coords = vert.tex_coords;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}