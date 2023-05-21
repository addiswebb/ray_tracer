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
    number_of_bounces: i32,
    rays_per_pixel: i32,
    toggle: i32,
};

@group(0) @binding(0)
var<uniform> params: Params;
@group(0) @binding(1)
var texture: texture_storage_2d<rgba32float,read_write>;

@fragment
fn frag(i: VertexOutput) -> @location(0) vec4<f32>{
    var color = textureLoad(texture, vec2<i32>(
        i32(i.tex_coord.x * f32(params.width)),
        i32(i.tex_coord.y * f32(params.height))
    ));
    if (params.toggle != 0){
        return vec4<f32>(color.rgb,color.a);
    }else{
        return vec4<f32>(pow(color.rgb,vec3<f32>(2.2)),color.a);
    }

}