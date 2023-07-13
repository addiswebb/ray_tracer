use std::{time::Duration, mem};

use bytemuck::{Pod, Zeroable};
use imgui_winit_support::winit;
use wgpu::{SurfaceConfiguration, util::DeviceExt};

use super::{imgui::ImguiLayer, texture::Texture, context::Params};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TexVertex {
    _pos: [f32; 4],
    _tex_coord: [f32; 2],
}

pub struct Renderer{
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub imgui_layer: ImguiLayer,
    pub dt: Duration,
}
impl Renderer{
    fn vertex(pos: [i8; 2], tc: [i8; 2]) -> TexVertex {
        TexVertex {
            _pos: [pos[0] as f32, pos[1] as f32, 1.0, 1.0],
            _tex_coord: [tc[0] as f32, tc[1] as f32],
        }
    }
    fn create_vertices() -> (Vec<TexVertex>, Vec<u16>) {

        let vertex_data = [
            Renderer::vertex([-1,-1], [1, 0]),
            Renderer::vertex([ 1,-1], [0, 0]),
            Renderer::vertex([ 1, 1], [0, 1]),
            Renderer::vertex([-1, 1], [1, 1]),
        ];

        let index_data: &[u16] = &[
            0, 1, 2, 2, 3, 0,
        ];

        (vertex_data.to_vec(), index_data.to_vec())
    }
    //maybe change texture to texture_binding resource
    pub async fn new(device: &wgpu::Device,queue: &wgpu::Queue,texture: &Texture, config: &SurfaceConfiguration, params_buffer: &wgpu::Buffer, window_ref: &winit::window::Window) -> Self{
        
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor{
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/render.wgsl").into()),
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

        let (vertices, indices) = Renderer::create_vertices();
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

        let imgui_layer = ImguiLayer::new(window_ref, &config, &device, &queue).await;

        let dt = Duration::new(0,0);
        Self {
            pipeline,
            bind_group, 
            bind_group_layout, 
            vertex_buffer, 
            index_buffer, 
            imgui_layer,
            dt, 
        }
    }
}