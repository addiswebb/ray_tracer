use std::{mem, borrow::BorrowMut};

use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec4};
use imgui_winit_support::winit::{self, event::{WindowEvent, KeyboardInput, ElementState, MouseButton, VirtualKeyCode}};
use wgpu::util::DeviceExt;

use super::{window::Window, imgui::ImguiLayer, texture::Texture, camera::{Camera, }};

const WORKGROUP_SIZE: (u32, u32) = (8, 8);

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    _pos: [f32; 4],
    _tex_coord: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable,Debug)]
pub struct Params {
    width : u32,
    height : u32,
    number_of_bounces: i32,
    rays_per_pixel: i32,
}


#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct Sphere{
    position: [f32;3], 
    radius: f32,
    color: [f32;4],
    emission_color: [f32;4],
    emission_strength: f32,
    _padding: [f32;3],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct Material{
    color: [f32;4],
    emission_color: [f32;4],
    emission_strength: f32,
}

impl Sphere{
    pub fn new(position: Vec3, radius: f32, color: Vec4, emission_color: Vec4, emission_strength: f32) -> Self{
        Self { 
            position: position.to_array(),
            radius,
            color: color.to_array(),
            emission_color: emission_color.to_array(),
            emission_strength,
            _padding: [123.0;3],
        }
    }
}


pub struct Context{
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    pub config: wgpu::SurfaceConfiguration,
    pub pipeline: wgpu::RenderPipeline,
    pub imgui_layer: ImguiLayer,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub compute_pipeline: wgpu::ComputePipeline,
    pub compute_bind_group: wgpu::BindGroup,
    pub compute_bind_group_layout: wgpu::BindGroupLayout,
    pub texture: Texture,
    pub params_buffer: wgpu::Buffer,
    pub params: Params,
    pub camera: Camera,
    pub scene_buffer: wgpu::Buffer,
    pub mouse_pressed: bool,
}

impl Context{
    fn vertex(pos: [i8; 2], tc: [i8; 2]) -> Vertex {
        Vertex {
            _pos: [pos[0] as f32, pos[1] as f32, 1.0, 1.0],
            _tex_coord: [tc[0] as f32, tc[1] as f32],
        }
    }
    fn create_vertices() -> (Vec<Vertex>, Vec<u16>) {
/*         let vertex_data = [
            Context::vertex([-1,-1], [0, 0]),
            Context::vertex([ 1,-1], [1, 0]),
            Context::vertex([ 1, 1], [1, 1]),
            Context::vertex([-1, 1], [0, 1]),
        ]; */

        let vertex_data = [
            Context::vertex([-1,-1], [1, 0]),
            Context::vertex([ 1,-1], [0, 0]),
            Context::vertex([ 1, 1], [0, 1]),
            Context::vertex([-1, 1], [1, 1]),
        ];

        let index_data: &[u16] = &[
            0, 1, 2, 2, 3, 0,
        ];

        (vertex_data.to_vec(), index_data.to_vec())
    }

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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor{
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/render.wgsl").into()),
        });
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor{
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/ray_tracer.wgsl").into())
        });

        let (vertices, indices) = Context::create_vertices();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        println!("{} {}", config.width, config.height);
        let params = Params {
            width: config.width,
            height: config.height,
            number_of_bounces: 1,
            rays_per_pixel: 1,
        };
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("parameters buffer"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let texture = Texture::new(&device,config.width,config.height,wgpu::TextureFormat::Rgba32Float);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry{
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(mem::size_of::<Params>() as _),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: texture.binding_type(wgpu::StorageTextureAccess::ReadWrite),
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: texture.binding_resource(),
                },
            ],
            label: Some("Bind Group"),
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
            label: Some(&format!("{:?}", shader)),
            layout: Some(&layout),
            vertex: wgpu::VertexState{
                module: &shader,
                entry_point: "vert",
                buffers: &[
                    wgpu::VertexBufferLayout{
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0=> Float32x4,1=>Float32x2],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "frag",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        alpha: wgpu::BlendComponent::REPLACE,
                        color: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState { 
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let camera = Camera::new(&device,Vec3::new(2.2,0.0,-3.7),Vec3::new(1.5,0.0,-3.0),Vec3::new(0.0,1.0,0.0),45.0,config.width as f32/config.height as f32,0.1,100.0);
/*         let camera = Camera::new(&device,Vec3::new(-5.0,-10.0,0.0),Vec3::new(-2.0,-3.0,0.0),Vec3::new(0.0,1.0,0.0),45.0,config.width as f32/config.height as f32,0.1,100.0); */
/*         let camera = Camera::new(&device,Vec3::new(-2.7,1.3,-8.0),Vec3::new(-2.6,1.0,-7.0),Vec3::new(0.0,1.0,0.0),28.0,config.width as f32/config.height as f32,0.1,100.0); */
        println!("{} {}",camera.pitch, camera.yaw);

        let scene = [
            Sphere::new(
                Vec3::new(-2.54,-0.72,0.5),0.6, 
                Vec4::new(1.0,0.0,0.0,1.0),
                Vec4::new(0.0,0.0,0.0,1.0),0.0,
            ),
            Sphere::new(
                Vec3::new(-1.27,-0.72,1.0),0.5, 
                Vec4::new(0.0,1.0,0.0,1.0),
                Vec4::new(1.0,1.0,1.0,1.0), 0.0,
            ),
            Sphere::new(
                Vec3::new(-0.5,-0.9,1.55),0.35,
                Vec4::new(0.0,0.0,1.0,1.0),
                Vec4::new(1.0,1.0,1.0,1.0), 0.0,
            ),
            /*  floor*/
            Sphere::new(
                Vec3::new(-3.46,-15.88,2.76),15.0,
                Vec4::new(0.5,0.0,0.8,1.0),
                Vec4::new(1.0,1.0,1.0,1.0), 0.0,
            ),
            /*  Light Object       */
            Sphere::new(
                Vec3::new(-7.44,-0.72,20.0),15.0,
                Vec4::new(0.1,0.1,0.1,0.0),
                Vec4::new(1.0,1.0,1.0,1.0), 2.0,
            )
        ];

        let scene_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Scene Buffer"),
            contents: bytemuck::bytes_of(&scene),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST| wgpu::BufferUsages::STORAGE,
        }); 

        let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Compute Bind Group Layout"),
            entries: &[
                //Params
                wgpu::BindGroupLayoutEntry{
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(mem::size_of::<Params>() as _),
                    },
                    count: None,
                },
                //Camera
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer{
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                //Texture
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: texture.binding_type(wgpu::StorageTextureAccess::WriteOnly),
                    count: None,
                },
                //Scene
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer{
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new((mem::size_of::<Sphere>() * scene.len()) as _),
                    },
                    count: None,
                },
            ],
        });
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: Some("Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry{
                    binding: 1,
                    resource: camera.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: texture.binding_resource(),
                },
                wgpu::BindGroupEntry{
                    binding: 3,
                    resource: scene_buffer.as_entire_binding(),
                },
            ],
        });

        let compute_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{
            label: Some("Compute Pipeline layout"),
            bind_group_layouts: &[&compute_bind_group_layout],
            push_constant_ranges: &[],
        });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_layout),
            module: &compute_shader,
            entry_point: "main",
        });

        let imgui_layer = ImguiLayer::new(window.as_ref(), &config, &device, &queue).await;


        Self{
            device,
            queue,
            surface,
            config,
            pipeline,
            imgui_layer,
            vertex_buffer,
            index_buffer,
            bind_group,
            bind_group_layout,
            compute_pipeline,
            compute_bind_group,
            compute_bind_group_layout,
            texture,
            params_buffer,
            params,
            camera,
            scene_buffer,
            mouse_pressed: false,
        }
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>){
        if size.width > 0 && size.height > 0{
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);
            self.texture = Texture::new(&self.device,size.width,size.height,wgpu::TextureFormat::Rgba32Float);

            self.params.width = size.width;
            self.params.height = size.height;

            self.queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[self.params]));
            self.compute_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor{
                label: Some("Compute Bind Group"),
                layout: &self.compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.camera.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.texture.binding_resource(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: self.scene_buffer.as_entire_binding(),
                    },
                ],
            });

            self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.bind_group_layout,
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
        let io = self.imgui_layer.context.io();
        if io.want_capture_mouse || io.want_capture_keyboard {
            return false;
        } 
        match event{
            WindowEvent::KeyboardInput { 
                input: 
                    KeyboardInput{
                        virtual_keycode: Some(key),
                        state,
                        ..
                    }, 
                ..
            } => self.camera.controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera.controller.process_scroll(delta);
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
        }
    }
    pub fn update(&mut self){
        self.camera.update_camera();
        let uniform = self.camera.to_uniform();
        self.queue.write_buffer(&self.camera.buffer, 0, bytemuck::cast_slice(&[uniform]));
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

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
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
            
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group,&[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1);

            let ui = self.imgui_layer.context.frame();
            {
                ui.window("Camera Info")
                    .size([200.0, 100.0], imgui::Condition::FirstUseEver)
                    .build(|| {
                        ui.text(format!(
                            "Position: ({})",
                            self.camera.origin
                        ));
                        ui.text(format!(
                            "Look At: ({})",
                            self.camera.look_at
                        ));
                        ui.text(format!(
                            "pitch: ({})",
                            self.camera.pitch
                        ));
                        ui.text(format!(
                            "yaw: ({})",
                            self.camera.yaw
                        ));
                        ui.input_int("Number of bounces: ", &mut self.params.number_of_bounces).build();
                        ui.input_int("Rays per pixel: ", &mut self.params.rays_per_pixel).build();
                    });
            }

            self.imgui_layer
            .render(&self.device, &self.queue, &mut render_pass)
            .expect("Failed to render imgui layer");
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}