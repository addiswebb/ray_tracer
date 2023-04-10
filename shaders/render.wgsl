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

struct Params{
    width: u32,
    height: u32,
};

@group(0) @binding(0)
var<uniform> params: Params;
@group(0) @binding(1)
var texture: texture_storage_2d<rgba32float,read_write>;

@fragment
fn frag(i: VertexOutput) -> @location(0) vec4<f32>{
    var coord: vec2<i32> = vec2<i32>(i32(i.tex_coord.x * f32(params.width)),i32(i.tex_coord.y * f32(params.height)));
    var color: vec4<f32> = textureLoad(texture, coord);
    return color;
}