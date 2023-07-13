use std::time::{Duration, Instant};
use bytemuck::{Pod, Zeroable};
use imgui_winit_support::winit::{self, event::{WindowEvent, KeyboardInput, ElementState, MouseButton }};
use wgpu::{util::DeviceExt};

use crate::core::{renderer::Renderer, ray_tracer::RayTracer};
use super::{window::Window, texture::Texture, scene::Scene};

const WORKGROUP_SIZE: (u32, u32) = (8, 8);

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable,Debug)]
pub struct Params {
    width : u32,
    height : u32,
    number_of_bounces: i32,
    rays_per_pixel: i32,
    skybox: i32,
    frames: i32,
    accumulate: i32,
}

pub struct Context{
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    pub config: wgpu::SurfaceConfiguration,
    pub texture: Texture,
    pub params_buffer: wgpu::Buffer,
    pub params: Params,
    pub renderer: Renderer,
    pub ray_tracer: RayTracer,
    pub scene: Scene,
    pub mouse_pressed: bool,
}

impl Context{
    pub async fn new(window: &Window) -> Self{
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor{
            backends: wgpu::Backends::VULKAN,
            dx12_shader_compiler: Default::default(),
        });

        let size = window.size();

        let surface = unsafe {instance.create_surface(&window.raw)}.unwrap();
        let mut config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            // using an erroneous format, it is changed before used
            format: wgpu::TextureFormat::Depth24Plus,
            width: size.x,
            height: size.y,
            present_mode: wgpu::PresentMode::Mailbox,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions{
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface)
        }).await.unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor{
                features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                ..Default::default()
            }, None)
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps.formats.iter()
            .copied()
            .filter(|f| f.describe().srgb)
            .next()
            .unwrap_or(surface_caps.formats[0]);
        config.format       = format;
        config.alpha_mode   = surface_caps.alpha_modes[0];
        config.present_mode = surface_caps.present_modes[0];
        surface.configure(&device, &config);

        println!("{} {}", config.width, config.height);
        let params = Params {
            width: config.width,
            height: config.height,
            number_of_bounces: 1,
            rays_per_pixel: 1,
            skybox: 1,
            frames: 0,
            accumulate: 1,
        };
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("parameters buffer"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let texture = Texture::new(&device,config.width,config.height,wgpu::TextureFormat::Rgba32Float);

        let renderer = Renderer::new(&device,&queue,&texture,&config,&params_buffer,window.as_ref()).await;

        let scene = Scene::new(&device, &config).await;

        let ray_tracer = RayTracer::new(&device,&texture, &params_buffer, &scene);

        Self{
            device,
            queue,
            surface,
            config,
            texture,
            params_buffer,
            params,
            renderer,
            ray_tracer,
            scene,
            mouse_pressed: false,
        }
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>){
        if size.width > 0 && size.height > 0{
            self.config.width = size.width;
            self.config.height = size.height;
            self.scene.camera.aspect = size.width as f32/ size.height as f32;
            self.surface.configure(&self.device, &self.config);
            self.texture = Texture::new(&self.device,size.width,size.height,wgpu::TextureFormat::Rgba32Float);

            self.params.width = size.width;
            self.params.height = size.height;
            self.params.frames = -1;

            self.queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[self.params]));
            self.ray_tracer.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor{
                label: Some("Compute Bind Group"),
                layout: &self.ray_tracer.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.scene.camera.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.texture.binding_resource(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: self.scene.spheres.1.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: self.scene.vertices.1.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: self.scene.indices.1.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: self.scene.meshes.1.as_entire_binding(),
                    },
                ],
            });

            self.renderer.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.renderer.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry{
                        binding: 0,
                        resource: self.params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.texture.binding_resource(),
                    },
                ],
                label: Some("Bind Group"),
            });
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        let io = self.renderer.imgui_layer.context.io();
        if io.want_capture_mouse || io.want_capture_keyboard {
            return false;
        } 
        let moved = match event{
            WindowEvent::KeyboardInput { 
                input: 
                    KeyboardInput{
                        virtual_keycode: Some(key),
                        state,
                        ..
                    }, 
                ..
            } => self.scene.camera.controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.scene.camera.controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput { 
                button: MouseButton::Left, 
                state,
                ..
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            _ => false,
        };
        match moved{
            true => {
                self.params.frames = -1;
                //TODO remove these writes and rely on update for that
                self.queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[self.params]));
            },
            _ => {},
        }

        return moved;
    }
    pub fn update(&mut self, dt: Duration){
        self.renderer.dt = dt;
        self.scene.camera.update_camera(self.renderer.dt);
        let uniform = self.scene.camera.to_uniform();
        if self.params.accumulate != 0{
            self.params.frames +=1;
        }else{
            self.params.frames = -1;
        }
        self.queue.write_buffer(&self.scene.camera.buffer, 0, bytemuck::cast_slice(&[uniform]));
        self.queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[self.params]));
    }
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError>{
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor{
            label: Some("Command Encoder")
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor{
                label: Some("Compute Pass"),
            });
            let xdim = self.config.width + WORKGROUP_SIZE.0 - 1;
            let xgroups = xdim / WORKGROUP_SIZE.0;
            let ydim = self.config.height + WORKGROUP_SIZE.1 - 1;
            let ygroups = ydim / WORKGROUP_SIZE.1;

            compute_pass.set_pipeline(&self.ray_tracer.pipeline);
            compute_pass.set_bind_group(0, &self.ray_tracer.bind_group, &[]);
            compute_pass.dispatch_workgroups(xgroups,ygroups,1);
        }
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor{
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment{
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations{
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            
            render_pass.set_pipeline(&self.renderer.pipeline);
            render_pass.set_bind_group(0, &self.renderer.bind_group,&[]);
            render_pass.set_vertex_buffer(0, self.renderer.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.renderer.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1);
            let mut skybox = self.params.skybox != 0;
            let mut accumulate = self.params.accumulate != 0;
            let ui = self.renderer.imgui_layer.context.frame();
            {
                ui.window("Camera Info")
                    .size([200.0, 100.0], imgui::Condition::FirstUseEver)
                    .build(|| {
                        ui.text(format!(
                            "Frame time: ({:#?})",
                            self.renderer.dt.as_millis() as f32
                        ));
                        ui.text(format!(
                            "Frame: {}",self.params.frames
                        ));
                        ui.text(format!(
                            "Position: ({})",
                            self.scene.camera.origin
                        ));
                        ui.text(format!(
                            "Look At: ({})",
                            self.scene.camera.look_at
                        ));
                        ui.input_int("Bounces", &mut self.params.number_of_bounces).build();
                        ui.input_int("Rays per pixel", &mut self.params.rays_per_pixel).build();
                        ui.checkbox("Skybox", &mut skybox);
                        ui.checkbox("Accumulate", &mut accumulate);
                    });
            }
            self.params.skybox = skybox as i32;
            self.params.accumulate = accumulate as i32;

            self.renderer.imgui_layer
            .render(&self.device, &self.queue, &mut render_pass)
            .expect("Failed to render imgui layer");
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}