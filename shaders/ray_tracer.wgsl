struct Params{
    width: u32,
    height: u32,
};
struct Material{
    color: vec4<f32>,
    emission_color: vec4<f32>,
    emission_strength: f32,
}

struct Sphere{
    position: vec3<f32>,
    radius: f32,
    material: Material,
};

struct Scene{
    spheres: array<Sphere>,
};

@group(0) @binding(0)
var<uniform> params: Params;
@group(0) @binding(1)
var<uniform> camera: Camera;
@group(0) @binding(2)
var texture: texture_storage_2d<rgba32float,write>;
@group(0) @binding(3)
var<storage,read> scene: array<Sphere>;

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
    material: Material,
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

fn calculate_ray_collions(ray: Ray) -> Hit{
    var closest_hit: Hit; 
    closest_hit.dst = 0x1.fffffep+127f;
    for(var i: u32 = 0u; i < arrayLength(&scene); i+=1u){
        var hit: Hit = ray_sphere(ray, scene[i].position, scene[i].radius);
        if hit.hit && hit.dst < closest_hit.dst{
            closest_hit = hit;
            closest_hit.material = scene[i].material;
        }
    }
    return closest_hit;
}

fn gen(state: u32) -> u32{
    let state = state * 747796405u + 2891336453u;
    var result = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    result = (result >> 22u) ^ result;
    return result;
}
fn rand(state: f32) -> f32{
    return f32(gen(u32(state))) / 4294967295.0;
}

fn rand_distribution(state: f32) -> f32{
    let theta = 2.0 * 3.1415926 * rand(state);
    let rho = sqrt(-2.0 * log(rand(state)));
    return rho * cos(theta);
}

fn rand_dir(state: f32) -> vec3<f32>{
    let x = rand_distribution(state);
    let y = rand_distribution(state);
    let z = rand_distribution(state);
    return normalize(vec3<f32>(x,y,z));
}

fn rand_hemisphere_dir(normal: vec3<f32>, state: f32) -> vec3<f32>{
    let dir = rand_dir(state);
    return dir * sign(dot(normal, dir));
}

fn trace(ray: Ray, rng_state: f32) -> vec4<f32>{
    var ray: Ray = ray;
    var incoming_light = vec4<f32>(0.0);
    var ray_color = vec4<f32>(1.0);
    for (var i = 0; i <= 1; i +=1){
        var hit: Hit = calculate_ray_collions(ray);
        if (hit.hit){
            ray.origin = hit.hit_point;
            ray.dir = rand_hemisphere_dir(hit.normal, rng_state);
            let emitted_light = hit.material.emission_color * hit.material.emission_strength;
            incoming_light += emitted_light * ray_color;
            ray_color *= hit.material.color;
        }else{
            break;
        }
    }
    return incoming_light;
}

fn frag(i: FragInput) -> vec4<f32>{
    let pixel_coord = i.pos * i.size;
    let pixel_index = pixel_coord.y * i.size.x + pixel_coord.x;

    let pos = i.pos / i.size;
    var ray: Ray;
    ray.origin = camera.origin;
    ray.dir = normalize(camera.lower_left_corner + pos.x * camera.horizontal + pos.y * camera.vertical - ray.origin);

    return trace(ray, pixel_index);
}