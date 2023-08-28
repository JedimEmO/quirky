
struct VertexInput {
    @location(0) pos: vec2<f32>
};
struct VertexOutput {
    @builtin(position) position: vec4<f32>
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.position = vec4<f32>(in.pos, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.1, 0.1, 0.8, 1.0);
}