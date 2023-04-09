struct VertexInput{
    @location(0) position: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

@vertex
fn vert(i: VertexInput) -> VertexOutput{
    var out: VertexOutput;
    out.position = i.position;
    out.tex_coord = i.tex_coord;
    return out;
}
@group(0) @binding(0)
var texture: texture_storage_2d<rgba32float,read_write>;

@fragment
fn frag(i: VertexOutput) -> @location(0) vec4<f32>{
    var coord: vec2<i32> = vec2<i32>(i32(i.tex_coord.x * 800.0),i32(i.tex_coord.y * 600.0));
    var color: vec4<f32> = textureLoad(texture, coord);
    return color;
}