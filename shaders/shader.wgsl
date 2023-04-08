struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>
}

@vertex
fn vert(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput{
    var out: VertexOutput;
    out.clip_position = vec4<f32>(f32(in_vertex_index));
    return out;
}

@fragment
fn frag(i: VertexOutput) -> @location(0) vec4<f32>{
    return vec4<f32>(1.0);
}