struct VertexInput{
    @location(0) position: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>
};

@vertex
fn vert(i: VertexInput) -> VertexOutput{
    var out: VertexOutput;
    out.clip_position = vec4<f32>(i.position, 0.0,1.0);
    return out;
}

@fragment
fn frag(i: VertexOutput) -> @location(0) vec4<f32>{
    return vec4<f32>(0.0,0.0,0.0,0.0);
}