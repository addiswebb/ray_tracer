use crate::{core::{*, context::Context}};

use imgui_winit_support::winit::{event::{Event, WindowEvent, KeyboardInput, ElementState, VirtualKeyCode}, event_loop::ControlFlow};
use window::Window;

mod core;

pub async fn run(){
    env_logger::builder()
        .filter_module("ray_tracer", log::LevelFilter::Info)
        .filter_module("wgpu_core", log::LevelFilter::Warn)
        .init();
    log::info!("Starting Ray Tracer");
    log::info!("Creating Window: 800x600");
    let window: Window = Window::new().title("Ray Tracer").size(800,600).build();

    let mut context = Context::new(&window).await;

    log::info!("Starting event_loop");
    window.event_loop.run(move |event, _, control_flow| {
        match event{
            Event::MainEventsCleared => window.raw.request_redraw(),
            Event::WindowEvent{ ref event, window_id, } if window_id == window.raw.id() => 
                if !context.input(&event){
                    match event {
                        WindowEvent::CloseRequested | WindowEvent::KeyboardInput{
                            input: KeyboardInput{
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            context.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            context.resize(**new_inner_size);
                        },
                        _ => {}
                    }
                }
            Event::RedrawRequested(window_id) if window_id == window.raw.id() => {
                context.update();
                match context.render(){
                    Ok(_) => {},
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        context.resize(window.raw.inner_size())
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            _ => (),
        }
        context.imgui_layer.event(&window.raw,&event);
    });
}