struct Params{
    width: u32,
    height: u32,
};

@group(0) @binding(0)
var<uniform> params: Params;
@group(0) @binding(1)
var<uniform> camera: Camera;
@group(0) @binding(2)
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
    origin: vec3<f32>,
    lower_left_corner: vec3<f32>,
    horizontal: vec3<f32>,
    vertical: vec3<f32>,
    near: f32,
    far: f32,
}

struct FragInput{
    pos: vec2<f32>,
    size: vec2<f32>,
};

struct Ray{
    origin: vec3<f32>,
    dir: vec3<f32>,
}

struct Hit{
    hit: bool,
    dst: f32,
    hit_point: vec3<f32>,
    normal: vec3<f32>, 
}

fn ray_sphere(ray: Ray, pos: vec3<f32>, radius: f32) -> Hit{
    var hit: Hit;
    let oc = ray.origin - pos;
    let a = dot(ray.dir,ray.dir);
    let b = 2.0 * dot(oc, ray.dir);
    let c = dot(oc,oc) - pow(radius,2.0);
    let discriminant = b * b - 4.0 * a * c;
    if discriminant >= 0.0{
        let dst = (-b - sqrt(discriminant))/(2.0*a);
        if dst >= 0.0{
            hit.hit = true;
            hit.hit_point = ray.origin + ray.dir * dst;
            hit.dst = dst;
            hit.normal = normalize(hit.hit_point - pos);
        }
    } 
    return hit;
}


fn frag(i: FragInput) -> vec4<f32>{
    let pos = i.pos / i.size;
    var ray: Ray;
    ray.origin = camera.origin;
    ray.dir = normalize(camera.lower_left_corner + pos.x * camera.horizontal + pos.y * camera.vertical - ray.origin);
    let hit = ray_sphere(ray, vec3<f32>(0.0,0.0,0.0), 0.8);
    var color = vec4<f32>(0.0,0.0,0.0,0.0);
    if hit.hit {
        color = vec4<f32>(1.0,1.0,1.0,0.0);
    }
    let x = dot(hit.normal, vec3<f32>(1.0,0.0,0.0));
    return color;

}