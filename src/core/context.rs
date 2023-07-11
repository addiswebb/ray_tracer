use std::{mem, time::Duration, path::Path};

use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec4};
use gltf::json::extensions::mesh;
use imgui_winit_support::winit::{self, event::{WindowEvent, KeyboardInput, ElementState, MouseButton }};
use wgpu::util::DeviceExt;

use crate::core::resource::load_model;

use super::{window::Window, imgui::ImguiLayer, texture::Texture, camera::{Camera, }};

const WORKGROUP_SIZE: (u32, u32) = (8, 8);

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TexVertex {
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
    toggle: i32,
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
pub struct Mesh{
    pub first: u32,
    pub triangles: u32,
    pub offset: u32,
    pub _padding2: f32,
    pub pos: [f32;3],
    pub _padding: f32,
    pub color: [f32;4],
    pub emission_color: [f32;4],
    pub emission_strength: f32,
    pub _padding3: [f32;3],
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

impl Mesh{
    pub fn new(pos: Vec3, first: u32, triangles: u32,offset: u32,color: Vec4, emission_color: Vec4, emission_strength: f32)->Self{
        Self{
            first,
            triangles,
            offset,
            _padding2: 0.0,
            pos: pos.to_array(),
            _padding: 0.0,
            color: color.to_array(),
            emission_color: emission_color.to_array(),
            emission_strength,
            _padding3: [0.0;3],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Vertex{
    pub pos: [f32;3],
    pub _padding1: f32,
    pub normal: [f32;3],
    pub _padding2: f32,
}

impl Vertex{
    pub fn new(pos: Vec3, normal: Vec3) -> Self{
        Self { pos: pos.to_array(), _padding1: 0.0, normal: normal.to_array(), _padding2: 0.0 }
    }
}

pub struct Context{
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    pub config: wgpu::SurfaceConfiguration,
    pub pipeline: wgpu::RenderPipeline,
    pub imgui_layer: ImguiLayer,
    pub tex_vertex_buffer: wgpu::Buffer,
    pub tex_index_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub compute_pipeline: wgpu::ComputePipeline,
    pub compute_bind_group: wgpu::BindGroup,
    pub compute_bind_group_layout: wgpu::BindGroupLayout,
    pub texture: Texture,
    pub params_buffer: wgpu::Buffer,
    pub params: Params,
    pub camera: Camera,
    pub sphere_buffer: wgpu::Buffer,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub mesh_buffer: wgpu::Buffer,
    pub mouse_pressed: bool,
    pub dt: Duration,
}

impl Context{
    fn vertex(pos: [i8; 2], tc: [i8; 2]) -> TexVertex {
        TexVertex {
            _pos: [pos[0] as f32, pos[1] as f32, 1.0, 1.0],
            _tex_coord: [tc[0] as f32, tc[1] as f32],
        }
    }
    fn create_vertices() -> (Vec<TexVertex>, Vec<u16>) {

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
        let tex_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let tex_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
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
            toggle: 1,
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
                        array_stride: std::mem::size_of::<TexVertex>() as wgpu::BufferAddress,
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

        let camera = Camera::new(&device,
            Vec3::new(-2.764473, 5.8210998, 3.839141),
            Vec3::new(-2.0999293, 5.1703076, 3.4719195),
            Vec3::new(0.0,1.0,0.0),45.0,
            config.width as f32/config.height as f32,0.1,100.0
        );
/*         let camera = Camera::new(&device,Vec3::new(-5.0,-10.0,0.0),Vec3::new(-2.0,-3.0,0.0),Vec3::new(0.0,1.0,0.0),45.0,config.width as f32/config.height as f32,0.1,100.0); */
/*         let camera = Camera::new(&device,Vec3::new(-2.7,1.3,-8.0),Vec3::new(-2.6,1.0,-7.0),Vec3::new(0.0,1.0,0.0),28.0,config.width as f32/config.height as f32,0.1,100.0); */
        println!("{} {}",camera.pitch, camera.yaw);

        let spheres= [
            Sphere::new(
                Vec3::new(-3.64,-0.72,0.8028),0.75, 
                Vec4::new(1.0,1.0,1.0,1.0),
                Vec4::new(0.0,0.0,0.0,1.0),0.0,
            ),
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

        #[allow(unused_mut)]
        let mut vertices = vec![
            Vertex::new(Vec3::new(0.0, 0.0, 1.0), Vec3::new(2.0,-3.0,-1.0)),
            Vertex::new(Vec3::new(0.0, 0.15, 0.0), Vec3::new(4.0,-3.0, 0.0)),
            Vertex::new(Vec3::new(1.0, 0.3, 0.0), Vec3::new(3.0,-4.0, 2.0)),
        ];
        #[allow(unused_mut)]
        let mut indices = vec![
            2u32,1u32,0u32,
            // 3u32,4u32,5u32
        ];
        #[allow(unused_mut)]
        let mut meshes = vec![
            Mesh::new(
                Vec3::new(0.0,0.0,0.0),
                0, 1, 0,
                Vec4::new(0.0,0.6,0.0,1.0),
                Vec4::new(1.0,1.0,1.0,1.0), 0.0,
            ),
            // Mesh::new(
            //     Vec3::new(0.0,0.0,0.0),
            //     3, 1, 6,
            //     Vec4::new(0.6,0.1,0.1,1.0),
            //     Vec4::new(1.0,1.0,1.0,1.0), 0.0,
            // ),
        ];

        load_model(Path::new("cube2.obj"),&mut vertices, &mut indices, &mut meshes).await.unwrap();
        load_model(Path::new("simple_cube.obj"),&mut vertices, &mut indices, &mut meshes).await.unwrap();
        //load_model(Path::new("poly_sphere.obj"),&mut vertices, &mut indices, &mut meshes).await.unwrap();

        for _ in 0..2{
            //load_model(Path::new("cube2.obj"), &mut vertices, &mut indices, &mut meshes).await.unwrap();
        }

        let sphere_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Sphere Buffer"),
            contents: bytemuck::bytes_of(&spheres),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST| wgpu::BufferUsages::STORAGE,
        }); 

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST| wgpu::BufferUsages::STORAGE,
        }); 

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST| wgpu::BufferUsages::STORAGE,
        }); 
        let mesh_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("Mesh Buffer"),
            contents: bytemuck::cast_slice(&meshes),
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
                //Spheres
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer{
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new((mem::size_of::<Sphere>() * spheres.len()) as _),
                    },
                    count: None,
                },
                //Vertex buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer{
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new((mem::size_of::<Vertex>() * vertices.len()) as _),
                    },
                    count: None,
                },
                //Index buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer{
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new((mem::size_of::<u32>() * indices.len()) as _),
                    },
                    count: None,
                },
                //Meshes
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer{
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new((mem::size_of::<Mesh>() * meshes.len()) as _),
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
                    resource: sphere_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry{
                    binding: 4,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry{
                    binding: 5,
                    resource: index_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry{
                    binding: 6,
                    resource: mesh_buffer.as_entire_binding(),
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

        let dt = Duration::new(0,0);
        Self{
            device,
            queue,
            surface,
            config,
            pipeline,
            imgui_layer,
            tex_vertex_buffer,
            tex_index_buffer,
            bind_group,
            bind_group_layout,
            compute_pipeline,
            compute_bind_group,
            compute_bind_group_layout,
            texture,
            params_buffer,
            params,
            camera,
            sphere_buffer,
            vertex_buffer,
            index_buffer,
            mesh_buffer,
            mouse_pressed: false,
            dt
        }
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>){
        if size.width > 0 && size.height > 0{
            self.config.width = size.width;
            self.config.height = size.height;
            self.camera.aspect = size.width as f32/ size.height as f32;
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
                        resource: self.sphere_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: self.vertex_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: self.index_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: self.mesh_buffer.as_entire_binding(),
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
    pub fn update(&mut self, dt: Duration){
        self.dt = dt;
        self.camera.update_camera(self.dt);
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
            render_pass.set_vertex_buffer(0, self.tex_vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.tex_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1);
            let mut toggle = self.params.toggle != 0;
            let ui = self.imgui_layer.context.frame();
            {
                ui.window("Camera Info")
                    .size([200.0, 100.0], imgui::Condition::FirstUseEver)
                    .build(|| {
                        ui.text(format!(
                            "Frame time: ({:#?})",
                            1000.0 /self.dt.as_millis() as f32
                        ));
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
                        ui.input_int("Bounces", &mut self.params.number_of_bounces).build();
                        ui.input_int("Rays per pixel", &mut self.params.rays_per_pixel).build();
                        ui.checkbox("Skybox", &mut toggle);
                    });
            }
            self.params.toggle = toggle as i32;

            self.imgui_layer
            .render(&self.device, &self.queue, &mut render_pass)
            .expect("Failed to render imgui layer");
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}