use glam::{Vec3};
use imgui_winit_support::winit::{event::{VirtualKeyCode, ElementState, MouseScrollDelta}, dpi::PhysicalPosition};
use wgpu::util::DeviceExt;
const SAFE_FRAC_PI_2: f32 = std::f32::consts::FRAC_PI_2 - 0.0001;
const SAFE_FRAC_PI_2_DEG: f32 = SAFE_FRAC_PI_2 * (180.0/std::f32::consts::PI);

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct CameraUniform{
    pub origin: [f32;3],
    _padding1: f32,
    pub lower_left_corner: [f32;3],
    _padding2: f32,
    pub horizontal: [f32;3],
    _padding3: f32,
    pub vertical: [f32;3],
    _padding4: f32,
    pub near: f32,
    pub far: f32,
    _padding5: [f32;2],
}

pub struct Camera{
    pub origin: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub look_at: Vec3,
    pub view_up: Vec3,
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pub buffer: wgpu::Buffer,
    pub controller: CameraController,
}
impl Camera{
    pub fn new(device: &wgpu::Device, origin: Vec3, look_at: Vec3, view_up: Vec3, fov: f32, aspect: f32, near: f32, far: f32) -> Self {
        let uniform = CameraUniform::default();
        
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Camera buffer"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let controller = CameraController::new(0.01,0.002);


        let pitch = 0.0;
        let yaw = 0.0;

        Camera{
            origin,
            pitch,yaw,
            look_at,
            view_up,
            fov,
            aspect,
            near,
            far,
            buffer,
            controller,
        }
    }
    pub fn to_uniform(&mut self) -> CameraUniform{
        let theta = radians(self.fov); 
        let half_height = self.near * f32::tan(theta/ 2.0);
        let half_width = self.aspect * half_height;
        let w = (self.origin - self.look_at).normalize();
        let u = self.view_up.cross(w).normalize();
        let v = w.cross(u);
        let horizontal = 2.0 * half_width * u;
        let vertical = 2.0 * half_height * v;
        let lower_left_corner = self.origin - half_width * u - half_height * v - self.near * w;

        CameraUniform {
            origin: self.origin.to_array(),
            _padding1: 0.0,
            lower_left_corner: lower_left_corner.to_array(),
            _padding2: 0.0,
            horizontal: horizontal.to_array(),
            _padding3: 0.0,
            vertical: vertical.to_array(),
            _padding4: 0.0,
            near: self.near, 
            far: self.far,
            _padding5: [0.0;2],
        }
    }
    pub fn update_camera(&mut self) {
        let direction = (self.look_at - self.origin).normalize();
        let mut pitch = direction.y.asin();
        let mut yaw = direction.x.atan2(direction.z);

        // Move forward/backward and left/right
        let (yaw_sin, yaw_cos) = yaw.sin_cos();
        let forward = Vec3::new(yaw_sin, 0.0, yaw_cos).normalize();
        let right = Vec3::new(yaw_cos, 0.0, -yaw_sin).normalize();
        self.origin += forward * (self.controller.amount_forward - self.controller.amount_backward) * self.controller.speed;
        self.origin += right * (self.controller.amount_right - self.controller.amount_left) * self.controller.speed;

        // Move in/out (aka. "zoom")
        // Note: this isn't an actual zoom. The camera's position
        // changes when zooming. I've added this to make it easier
        // to get closer to an object you want to focus on.
        let (pitch_sin, pitch_cos) = pitch.sin_cos();
        let scrollward = Vec3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        self.origin -= scrollward * self.controller.scroll * self.controller.speed * self.controller.sensitivity;
        self.controller.scroll = 0.0;

        // Move up/down. Since we don't use roll, we can just
        // modify the y coordinate directly.
        self.origin.y += (self.controller.amount_up - self.controller.amount_down) * self.controller.speed;

        // Rotate
        yaw += self.controller.rotate_horizontal * self.controller.sensitivity;
        pitch += -self.controller.rotate_vertical * self.controller.sensitivity;

        // If process_mouse isn't called every frame, these values
        // will not get set to zero, and the camera will rotate
        // when moving in a non cardinal direction.
        self.controller.rotate_horizontal = 0.0;
        self.controller.rotate_vertical = 0.0;

        // Keep the camera's angle from going too high/low.
        if pitch < -SAFE_FRAC_PI_2_DEG {
            pitch = -SAFE_FRAC_PI_2_DEG;
        } else if pitch > SAFE_FRAC_PI_2_DEG {
            pitch = SAFE_FRAC_PI_2_DEG;
        }
        self.look_at = self.origin + Vec3::new(pitch.cos() * yaw.sin(), pitch.sin(), pitch.cos() * yaw.cos());
    }
}
#[derive(Debug)]
pub struct CameraController{
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,
}

impl CameraController{
    pub fn new(speed: f32, sensitivity: f32) -> Self{
        Self{
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool{
        let amount = if state == ElementState::Pressed {5.0} else {0.0};
        
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up=>{
                self.amount_forward = amount;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down =>{
                self.amount_backward = amount;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left =>{
                self.amount_left = amount;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right =>{
                self.amount_right= amount;
                true
            }
            VirtualKeyCode::Space => {
                self.amount_up = amount;
                true
            }
            VirtualKeyCode::LShift => {
                self.amount_down = amount;
                true
            }
            _ => false
        }
    }
    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64){
        self.rotate_horizontal = mouse_dx as f32 * 3.0;
        self.rotate_vertical = mouse_dy as f32 * 3.0;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta){
        self.scroll = -match delta{
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 10000.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition{
                y: scroll,
                ..
            }) => *scroll as f32,
        };
    }

     
}
pub fn radians(deg: f32)->f32{
    deg * (std::f32::consts::PI / 180.0)
}
pub fn degrees(rad: f32) -> f32{
    rad * (180.0/std::f32::consts::PI)
}