pub mod error;
pub mod graphics_state;
pub mod surface;
pub mod window;
pub mod window_manager;

use std::{hint::unreachable_unchecked, sync::Arc};

use winit::application::ApplicationHandler;

use crate::graphics_state::GraphicsState;

pub struct Piru {
    graphics_state: Option<Arc<GraphicsState>>,
}

impl Piru {
    pub const fn new() -> Self {
        Self {
            graphics_state: None,
        }
    }

    pub fn run(&mut self) {
        winit::event_loop::EventLoopBuilder::default()
            .build()
            .unwrap()
            .run_app(self)
            .unwrap()
    }
}

impl Default for Piru {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationHandler for Piru {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.graphics_state = Some(GraphicsState::new(Some(Box::new(
            event_loop.owned_display_handle(),
        ))));

        match self.graphics_state {
            Some(ref state) => {
                let (_, window) = pollster::block_on(
                    self.graphics_state
                        .as_ref()
                        .unwrap()
                        .window_manager
                        .lock()
                        .unwrap()
                        .create_window(event_loop),
                )
                .unwrap();

                window.window.request_redraw();
            }
            None => unsafe { unreachable_unchecked() },
        }
    }

    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        log::info!("Suspending rendering at request of system.");

        self.graphics_state = None;
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::WindowEvent;

        let Some(mut window_manager) = self
            .graphics_state
            .as_ref()
            .map(|state| state.window_manager.lock().unwrap())
        else {
            return;
        };

        let Some(window) = window_manager.get_window(window_id) else {
            // The window has already been closed.
            // Why are we still getting messages for it?
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                window_manager.remove_window(window_id).unwrap();

                if window_manager.all_windows_closed() {
                    log::info!("All windows have been closed, exiting.");
                    event_loop.exit();
                }
            }

            WindowEvent::RedrawRequested => {
                window.redraw().unwrap();
            }

            WindowEvent::Resized(size) => {
                unsafe {
                    (*Arc::as_ptr(window).cast_mut())
                        .resize(size.width, size.height)
                        .unwrap()
                };

                window.window.request_redraw();
            }

            WindowEvent::Occluded(is_occluded) => {
                if !is_occluded {
                    window.window.request_redraw();
                }
            }

            WindowEvent::ScaleFactorChanged { .. } => {}
            WindowEvent::Focused(_) => {}
            WindowEvent::Ime(_) => {}
            WindowEvent::CursorMoved { .. } => {}

            _ => {
                dbg!(event);
            }
        }
    }
}

fn main() {
    colog::default_builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    Piru::new().run();
}
