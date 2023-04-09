@group(0) @binding(0)
var texture : texture_storage_2d<rgba32float,write>;

@compute
@workgroup_size(8,8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var i: FragInput;
    i.pos = vec2<f32>(f32(global_id.x),f32(global_id.y));
    i.size = vec2<f32>(1000.0,750.0);
    textureStore(texture,
        vec2<i32>(i32(i.pos.x), i32(i.pos.y)),
        frag(i));
}

struct FragInput{
    pos: vec2<f32>,
    size: vec2<f32>,
};

fn frag(i: FragInput) -> vec4<f32>{
    return vec4<f32>(i.pos.x/i.size.x, i.pos.x/i.size.x, i.pos.x/i.size.x,1.0);
}