use crate::emulator::Emulator;
use crate::cpu::{Cycles};
use crate::ppu::{WIDTH, HEIGHT};

use std::{thread, time};

use log::error;
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};
use winit_input_helper::WinitInputHelper;

pub fn create_pixels(width: u32, height: u32, window: &Window) -> Pixels {
    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, window);
    Pixels::new(width, height, surface_texture).unwrap()
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
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let window = create_window(WIDTH, HEIGHT, "rmg-001".to_string(), &event_loop);
    let mut pixels = create_pixels(WIDTH, HEIGHT, &window);

    let mut emulator = Emulator::new();

    event_loop.run(move |event, _, control_flow| {
        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
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
                emulator.run(Cycles(70224), pixels.get_frame());

                // thread::sleep(time::Duration::from_millis(1));
                window.request_redraw();
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
