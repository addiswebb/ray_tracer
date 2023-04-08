use std::path::Path;

use imgui_wgpu::{RendererConfig, Renderer};
use imgui_winit_support::winit::{self, event::Event};

use super::resources::load_binary;

pub struct ImguiLayer{
    pub context: imgui::Context,
    pub platform: imgui_winit_support::WinitPlatform,
    pub renderer: imgui_wgpu::Renderer,
}

impl ImguiLayer{
    pub async fn new(
        window: &winit::window::Window,
        surface_config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: &wgpu::Queue
    ) -> Self{
        let mut context = imgui::Context::create();
        context.io_mut().config_flags |= imgui::ConfigFlags::DOCKING_ENABLE;
        
        let mut platform = imgui_winit_support::WinitPlatform::init(&mut context);
        /*   
            Uncomment to stop imgui from saving state when closed     
            context.set_ini_filename(None); 
        */

        let hidpi_factor = window.scale_factor();
        let font_size = (13.0 * hidpi_factor) as f32;
            
        context.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        //load fonts
        /* let font_data = load_binary(
            &Path::new("assets")
            .join("fonts")
            .join("proggyclean.ttf")
        ).await.unwrap();

        let segoe_ui_font = imgui::FontSource::TtfData {
            data: &font_data,
            size_pixels: font_size,
            config: Some(
                imgui::FontConfig{
                    oversample_h: 1,
                    pixel_snap_h: true,
                    size_pixels: font_size,
                    ..Default::default()
                }
            ), 
        };

        context.fonts().add_font(&[segoe_ui_font]); */

        //Setup imgui renderer
        let renderer_config = RendererConfig{
            texture_format: surface_config.format,
            depth_format: None,
            ..Default::default()
        };

        let renderer = Renderer::new(&mut context, &device, &queue, renderer_config);
        platform.attach_window(context.io_mut(), window, imgui_winit_support::HiDpiMode::Default);
        Self{
            context,
            platform,
            renderer,
        }
    }

    pub fn render<'r>(&'r mut self, device: &wgpu::Device, queue: &wgpu::Queue, render_pass: &mut wgpu::RenderPass<'r>) -> Result<(),wgpu::SurfaceError>{
        self.renderer
            .render(self.context.render(), &queue, &device, render_pass)
            .expect("Failed to render imgui");
        Ok(())
    }

    pub fn attach(&mut self, window: &winit::window::Window) {
        self.platform.attach_window(
            self.context.io_mut(),
            window,
            imgui_winit_support::HiDpiMode::Default,
        );
    }
    pub fn event(&mut self, window: &winit::window::Window, event: &Event<()>) -> bool {
        self.platform.handle_event(self.context.io_mut(), window, event);
        return true;
    }
} 