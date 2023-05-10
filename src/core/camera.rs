use glam::{Vec3, Vec4};
use imgui_winit_support::winit::event::{VirtualKeyCode, ElementState};
use wgpu::util::DeviceExt;
const SAFE_FRAC_PI_2: f32 = std::f32::consts::FRAC_PI_2 - 0.0001;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct CameraUniform{
    pub origin: [f32;3],
    _padding1: f32,
    pub lower_left_corner:[f32;3],
    _padding2: f32,
    pub horizontal: [f32;3],
    _padding3: f32,
    pub vertical: [f32;3],
    _padding4: f32,
    pub near: f32,
    pub far: f32,
    _padding5: [f32;2]
}
impl CameraUniform{
    pub fn new(origin: Vec3, lower_left_corner: Vec3, horizontal: Vec3, vertical: Vec3, near: f32, far: f32) -> Self{
        Self {
            origin: origin.to_array(),
            _padding1: 0.0,
            lower_left_corner: lower_left_corner.to_array(),
            _padding2: 0.0,
            horizontal: horizontal.to_array(),
            _padding3: 0.0,
            vertical: vertical.to_array(),
            _padding4: 0.0,
            near, 
            far,
            _padding5: [0.0;2],
        }
    }
}

pub struct Camera{
    pub origin: Vec3,
    pub look_at: Vec3,
    pub view_up: Vec3,
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pub buffer: wgpu::Buffer,
    pub uniform: CameraUniform,
}
impl Camera{
    pub fn new(device: &wgpu::Device, origin: Vec3, look_at: Vec3, view_up: Vec3, fov: f32, aspect: f32, near: f32, far: f32) -> Self {
        let uniform = CameraUniform::default();
        
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Camera buffer"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Camera{
            origin,
            look_at,
            view_up,
            fov,
            aspect,
            near,
            far,
            buffer,
            uniform,
        }
    }
    pub fn update_uniform(&mut self) {
        let theta = self.fov * std::f32::consts::PI / 180.0;
        let half_height = self.near * f32::tan(theta / 2.0);
        let half_width = self.aspect * half_height;
        let w = (self.origin- self.look_at).normalize();
        let u = self.view_up.cross(w).normalize();
        let v = w.cross(u);
        let horizontal = 2.0 * half_width * u;
        let vertical = 2.0 * half_height * v;
        let lower_left_corner = self.origin - half_width * u - half_height * v - self.near * w;
        self.uniform = CameraUniform::new(self.origin, lower_left_corner, horizontal,vertical, self.near, self.far);
    } 
    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState ) -> bool{
        let amount = if state == ElementState::Pressed {1.0} else {0.0};

        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up=>{
                self.look_at.y += amount;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down =>{
                self.look_at.y -= amount;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left =>{
                self.look_at.x -= amount;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right =>{
                self.look_at.x += amount;
                true
            }
            VirtualKeyCode::Space => {
                self.look_at.z += amount;
                true
            }
            VirtualKeyCode::LShift => {
                self.look_at.z -= amount;
                true
            }
            _ => false
        }
    }
    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64){
        self.origin.x += degrees(mouse_dx as f32)/10.0;
        self.origin.y += degrees(mouse_dy as f32)/10.0;

        if self.origin.y< -degrees(SAFE_FRAC_PI_2) {
            self.origin.y = -degrees(SAFE_FRAC_PI_2);
        } else if self.origin.y > degrees(SAFE_FRAC_PI_2) {
            self.origin.y = degrees(SAFE_FRAC_PI_2);
        }
    }
}

pub fn degrees(rad: f32) -> f32{
    rad * (180.0/std::f32::consts::PI)
}