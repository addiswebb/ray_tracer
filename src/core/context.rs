use std::mem;

use bytemuck::{Pod, Zeroable};
use imgui_winit_support::winit::{self, event::WindowEvent};
use wgpu::util::DeviceExt;

use super::{window::Window, imgui::ImguiLayer, texture::Texture};

const WORKGROUP_SIZE: (u32, u32) = (8, 8);

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    _pos: [f32; 4],
    _tex_coord: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable,Debug)]
struct Params {
    width : u32,
    height : u32,
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
    pub params: wgpu::Buffer,
}

impl Context{
    fn vertex(pos: [i8; 2], tc: [i8; 2]) -> Vertex {
        Vertex {
            _pos: [pos[0] as f32, pos[1] as f32, 1.0, 1.0],
            _tex_coord: [tc[0] as f32, tc[1] as f32],
        }
    }
    fn create_vertices() -> (Vec<Vertex>, Vec<u16>) {
        let vertex_data = [
            Context::vertex([-1,-1], [0, 0]),
            Context::vertex([ 1,-1], [1, 0]),
            Context::vertex([ 1, 1], [1, 1]),
            Context::vertex([-1, 1], [0, 1]),
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

        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("parameters buffer"),
            contents: bytemuck::bytes_of(&
                Params {
                    width: config.width,
                    height: config.height,
            }),
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
                    resource: params.as_entire_binding(),
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
                buffers: &[wgpu::VertexBufferLayout{
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=> Float32x4,1=>Float32x2],
                }],
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

        let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Compute Bind Group Layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: texture.binding_type(wgpu::StorageTextureAccess::WriteOnly),
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
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: texture.binding_resource(),
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
            params,
        }
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>){
        if size.width > 0 && size.height > 0{
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);
            self.texture = Texture::new(&self.device,size.width,size.height,wgpu::TextureFormat::Rgba32Float);
            let params = Params{
                width: size.width,
                height: size.height,
            };
            self.queue.write_buffer(&self.params, 0, bytemuck::cast_slice(&[params]));
            self.compute_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor{
                label: Some("Compute Bind Group"),
                layout: &self.compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.params.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.texture.binding_resource(),
                    },
                ],
            });

            self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry{
                        binding: 0,
                        resource: self.params.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.texture.binding_resource(),
                    },
                ],
                label: Some("Bind Group"),
            });
            // self.queue.write_buffer(&self.params, 0, bytemuck::cast_slice(&[
            //     Params{
            //         width:size.width,
            //         height:size.height
            //     }
            // ]));
        }
    }
    pub fn input(&mut self, _event: &WindowEvent) -> bool {
        let io = self.imgui_layer.context.io();
        if io.want_capture_mouse || io.want_capture_keyboard {
            return false;
        } 
        false
    }
    pub fn update(&mut self){
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
                ui.window("Hello world")
                    .size([150.0, 50.0], imgui::Condition::FirstUseEver)
                    .build(|| {
                        ui.text(format!(
                            "Params: ({:.1},{:.1})",
                            self.config.width, self.config.height
                        ));
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