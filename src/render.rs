use crate::emulator::Emulator;
use crate::frames::Frames;
use crate::cpu::Cycles;
use crate::ppu::{WIDTH, HEIGHT};

use log::error;
use pixels::{wgpu, Pixels, PixelsBuilder, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};
use winit_input_helper::WinitInputHelper;

pub fn create_pixels(width: u32, height: u32, window: &Window) -> Pixels {
    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, window);
    // Pixels::new(width, height, surface_texture).unwrap()
    PixelsBuilder::new(width, height, surface_texture)
        .device_descriptor(wgpu::DeviceDescriptor {
            limits: wgpu::Limits {
                max_storage_textures_per_shader_stage: 4,
                max_texture_dimension_2d: 4096,
                max_texture_dimension_1d: 4096,
                ..wgpu::Limits::default()
            },
            ..wgpu::DeviceDescriptor::default()
        })
        .enable_vsync(false)
        .build()
        .unwrap()
}

pub fn create_window<T>(width: u32, height: u32, title: String, event_loop: &EventLoop<T>) -> Window {
    let size = LogicalSize::new(width as f64, height as f64);
    WindowBuilder::new()
        .with_title(title)
        .with_inner_size(size)
        .with_min_inner_size(size)
        .build(event_loop)
        .unwrap()
}

pub fn start_eventloop() {
    let mut emulator = Emulator::new();
    let mut frame_counter = Frames::new();
    let mut frame_limit = Frames::new();

    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let window = create_window(WIDTH, HEIGHT, "rmg-001".to_string(), &event_loop);
    let mut pixels = create_pixels(WIDTH, HEIGHT, &window);

    event_loop.run(move |event, _, control_flow| {
        // *control_flow = ControlFlow::Wait;

        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                emulator.close();
                *control_flow = ControlFlow::Exit;
                return;
            }

            emulator.handle_input(&input);

            // Resize the window
            if let Some(size) = input.window_resized() {
                pixels.resize_surface(size.width, size.height);
            }
        }
        
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                println!("The close button was pressed; stopping");
                *control_flow = ControlFlow::Exit
            },
            Event::MainEventsCleared => {
                emulator.run(Cycles(70224.0), pixels.get_frame());
                frame_counter.increment();
                if frame_counter.elapsed_ms() >= 1000 {
                    window.set_title(&format!("rmg-001 (FPS: {})", frame_counter.count()));
                    frame_counter.reset_count();
                    frame_counter.reset_timer();
                }
                window.request_redraw();
                frame_limit.limit();
                frame_limit.reset_timer();
            },
            Event::RedrawRequested(_) => {
                if pixels
                    .render()
                    .map_err(|e| error!("pixels.render() failed: {}", e))
                    .is_err()
                {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            },
            _ => ()
        }
    });
}
