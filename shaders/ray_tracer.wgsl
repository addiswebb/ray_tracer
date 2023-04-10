struct Params{
    width: u32,
    height: u32,
};

@group(0) @binding(0)
var<uniform> params: Params;
@group(0) @binding(1)
var texture: texture_storage_2d<rgba32float,write>;

@compute
@workgroup_size(8,8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var i: FragInput;
    i.pos = vec2<f32>(f32(global_id.x),f32(global_id.y));
    i.size = vec2<f32>(f32(params.width),f32(params.height));
    textureStore(texture, vec2<i32>(i32(i.pos.x), i32(i.pos.y)), frag(i));
}

struct Camera{
    near: f32,
    fov: f32,
    aspect: f32,
}

struct FragInput{
    pos: vec2<f32>,
    size: vec2<f32>,
};
fn frag(i: FragInput) -> vec4<f32>{
    return vec4<f32>(i.pos/i.size,0.0,1.0);
}