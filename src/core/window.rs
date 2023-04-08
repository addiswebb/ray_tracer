use mint::Vector2;
use imgui_winit_support::winit;

pub struct Window{
    pub event_loop: winit::event_loop::EventLoop<()>,
    pub raw: winit::window::Window,
}

#[derive(Default)]
pub struct WindowBuilder{
    title: Option<String>,
    size: Option<wgpu::Extent3d>,
}

impl Window{
    pub fn new() -> WindowBuilder{
        WindowBuilder::default() 
    }
    pub fn size(&self) -> Vector2<u32>{
        let size = self.raw.inner_size();
        Vector2{x: size.width, y:size.height}
    }
    pub fn as_ref(&self) -> &winit::window::Window{
        &self.raw
    }
}

impl WindowBuilder{
    pub fn title(self, title: &str) -> Self{
        Self { 
            title: Some(title.to_string()), 
            ..self
        }
    }

    pub fn size(self, width: u32, height: u32) -> Self{
        Self{
            size: Some(wgpu::Extent3d { 
                width, 
                height, 
                depth_or_array_layers: 1 
            }),
            ..self
        }
    }

    pub fn build(self) -> Window{
        let event_loop = winit::event_loop::EventLoop::new();
        let mut builder = winit::window::WindowBuilder::new()
            .with_min_inner_size(winit::dpi::Size::Logical((64,64).into()));
        
        if let Some(title) = self.title{
            builder = builder.with_title(title);
        }
        if let Some(size) = self.size{
            builder = builder.with_inner_size(winit::dpi::Size::Logical((size.width, size.height).into()));
        }
        let raw = builder.build(&event_loop).unwrap();
        Window{event_loop,raw}
    }
}