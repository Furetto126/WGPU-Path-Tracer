use std::{sync::Arc, time::Instant};

use winit::{application::ApplicationHandler, event::{KeyEvent, WindowEvent}, event_loop::ActiveEventLoop, keyboard::PhysicalKey, window::Window};

use crate::renderer::RendererState;

pub struct App {
    state: Option<RendererState>,

    timer: f32,
    last_render_time: Instant,
    frame_count: u32,
    current_fps: f32
}

impl App {
    pub fn new() -> Self {
        print!("\x1B[2J");
        Self {
            state: None,
            timer: 0.0,
            last_render_time: Instant::now(),
            frame_count: 0,
            current_fps: 0.0
        }
    }
}

impl ApplicationHandler<RendererState> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes();
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        self.state = Some(pollster::block_on(RendererState::new(window)).unwrap());
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: RendererState) {
        self.state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(s) => s,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let delta_time = now.duration_since(self.last_render_time).as_secs_f32();
                self.last_render_time = now;

                self.timer += delta_time;
                self.frame_count += 1;

                if self.timer > 1.0 {
                    self.current_fps = self.frame_count as f32 / self.timer;
                    self.frame_count = 0;
                    self.timer -= 1.0;

                    println!("FPS: {:.2}", self.current_fps);
                }

                state.update();
                match state.render() {
                    Ok(_) => {},
                    Err(e) => {
                        log::error!("{e}");
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => state.handle_key(event_loop, code, key_state.is_pressed()),
            _ => {}
        }
    }
}