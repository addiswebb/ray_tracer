struct Params{
    width: u32,
    height: u32,
    number_of_bounces: i32,
    rays_per_pixel: i32,
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

const HORIZON: vec4<f32> = vec4<f32>(1.0,1.0,1.0,0.0);
const ZENITH: vec4<f32> = vec4<f32>(0.0788092, 0.36480793, 0.7264151, 0.0);
const GROUND: vec4<f32> = vec4<f32>(0.35,0.3,0.35, 0.0);

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
fn rand_2(state: ptr<function,u32>) -> f32{
    *state = 0xadb4a92du * *state + 1u;

    // Low bits have less randomness [L'ECUYER '99], so we'll shift the high
    // bits into the mantissa position of an IEEE float32, then mask with
    // the bit-pattern for 2.0
    let m = (*state >> 9u) | 0x40000000u;

    let f = bitcast<f32>(m);   // Range [2:4]
    return f - 3.0;  
}

fn rand(state: f32) -> f32{
    return f32(gen(u32(state))) / 4294967295.0;
}

fn rand_distribution(state: f32) -> f32{
    let theta = 2.0 * 3.1415926 * rand(state);
    let rho = sqrt(-2.0 * log(rand(state)));
    return rho * cos(theta);
}

fn rand_distribution_2(seed: ptr<function, u32>) -> f32{
    let theta = 2.0 * 3.1415926 * rand_2(seed);
    let rho = sqrt(-2.0 * log(rand_2(seed)));
    return rho * cos(theta);
}

fn rand_unit_sphere(state: f32) -> vec3<f32>{
    for (var i = 0; i <= 100; i+=1){
        let p = 2.0 * vec3<f32>(rand(state),rand(state * 2.0),rand(state * 3.0)) - 1.0;
        let sqr = pow(p, vec3<f32>(2.0));
        let length_sqr = sqr.x + sqr.y + sqr.z;
        if(sqrt(length_sqr) < 1.0){
            return p;
        }
    }
    return vec3<f32>(0.0);
}

fn rand_unit_sphere_2(state: ptr<function, u32>) -> vec3<f32>{
    for (var i = 0; i <= 100; i+=1){
        let p = 2.0 * vec3<f32>(rand_2(state),rand_2(state),rand_2(state)) - 1.0;
        let sqr = pow(p, vec3<f32>(2.0));
        let length_sqr = sqr.x + sqr.y + sqr.z;
        if(sqrt(length_sqr) < 1.0){
            return p;
        }
    }
    return vec3<f32>(0.0);
}
fn rand_hemisphere_dir_dist(normal: vec3<f32>, state: f32) -> vec3<f32>{
    let dir = rand_unit_sphere(state);
    return dir * sign(dot(normal, dir));
}

fn rand_hemisphere_dir_dist_2(normal: vec3<f32>, seed: ptr<function, u32>) -> vec3<f32>{
    let dir = rand_unit_sphere_2(seed);
    return dir * sign(dot(normal, dir));
}

fn trace(ray: Ray, seed: ptr<function, u32>) -> vec4<f32>{
    var ray: Ray = ray;
    var ray_color = vec4<f32>(1.0);
    var incoming_light = vec4<f32>(0.0);
    for (var i = 0; i <= params.number_of_bounces; i +=1){
        var hit = calculate_ray_collions(ray);
        if (hit.hit){
            ray.origin = hit.hit_point;
            ray.dir = rand_hemisphere_dir_dist_2(hit.normal, seed);
            let emitted_light = hit.material.emission_color * hit.material.emission_strength;
            incoming_light += emitted_light * ray_color;
            ray_color *= hit.material.color;
        }else{
            incoming_light += get_environment_light(ray) * ray_color;
            break;
        }
    }
    return incoming_light;
}

fn get_environment_light(ray: Ray) -> vec4<f32>{
    let sky_gradient_t = pow(smoothstep(0.0, 0.4, ray.dir.y), 0.35);
    let ground_to_sky_t = smoothstep(-0.01, 0.0, ray.dir.y);
    let sky_gradient = mix(HORIZON,ZENITH, sky_gradient_t);
    let sun = pow(max(0.0, dot(ray.dir, vec3<f32>(0.1,1.0,0.1))),500.0) * 200.0;
    let composite = mix(GROUND, sky_gradient,ground_to_sky_t) + sun * f32(ground_to_sky_t >=1.0);
    return composite;
}

fn frag(i: FragInput) -> vec4<f32>{
    let pixel_coord = i.pos * i.size;
    var rng_state = gen(u32(pixel_coord.y * i.size.x + pixel_coord.x));

    let pos = i.pos / i.size;
    var ray: Ray;
    ray.origin = camera.origin;
    ray.dir = normalize(camera.lower_left_corner + pos.x * camera.horizontal + pos.y * camera.vertical - ray.origin);

    var total_incoming_light = vec4<f32>(0.0);

    for (var i = 0; i <= params.rays_per_pixel; i+=1){
        total_incoming_light += trace(ray, &rng_state);
    } 

    return total_incoming_light/f32(params.rays_per_pixel);
}