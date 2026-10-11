pub mod error;
pub mod graphics_state;
pub mod node;
pub mod surface;
pub mod window;
pub mod window_manager;
pub mod world;

use std::fmt::Alignment::Right;
use std::{str::FromStr, sync::Arc};

use wgpu::Face::Back;
use winit::application::ApplicationHandler;

use dear_imgui_rs;
use winit::dpi::PhysicalSize;
use winit::event::MouseButton::Left;

use crate::error::RenderError;
use crate::graphics_state::GraphicsState;
use crate::node::Node;
use crate::world::World;

pub struct Piru {
    graphics_state: Option<Arc<GraphicsState>>,
    worlds: Vec<World>,

    last_frame: std::time::Instant,
    last_frame_times: Vec<f64>,
}

impl Piru {
    pub fn new() -> Self {
        Self {
            graphics_state: None,
            worlds: vec![],

            last_frame_times: Vec::with_capacity(120),
            last_frame: std::time::Instant::now(),
        }
    }

    pub fn run(&mut self) {
        winit::event_loop::EventLoopBuilder::default()
            .build()
            .unwrap()
            .run_app(self)
            .unwrap();
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
                    state
                        .window_manager
                        .lock()
                        .unwrap()
                        .create_window(event_loop),
                )
                .unwrap();
                let PhysicalSize::<u32> {
                    width: x,
                    height: y,
                } = window.window.inner_size();

                state
                    .with_state(|state| {
                        state
                            .imgui_context
                            .lock()
                            .unwrap()
                            .io_mut()
                            .set_display_size([x as f32, y as f32]);

                        Ok(())
                    })
                    .unwrap();

                window.window.request_redraw();
            }

            None => unreachable!(),
        }
    }

    fn suspended(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        log::info!("Suspending rendering at request of system.");

        self.graphics_state = None;
    }

    #[allow(clippy::arithmetic_side_effects)]
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

        let state = self
            .graphics_state
            .as_ref()
            .unwrap()
            .graphics_state_internal
            .state
            .read()
            .map_err(|_| RenderError::GpuPoisoned)
            .unwrap();

        let state = state
            .as_ref()
            .ok_or(RenderError::NotInitializedYet)
            .unwrap();

        let mut imgui_context = state.imgui_context.lock().unwrap();

        let imgui_io = imgui_context.io_mut();

        match event {
            WindowEvent::CloseRequested => {
                window_manager.remove_window(window_id).unwrap();

                if window_manager.all_windows_closed() {
                    log::info!("All windows have been closed, exiting.");
                    event_loop.exit();
                }
            }

            WindowEvent::RedrawRequested => {
                let now = std::time::Instant::now();

                let delta = (now - self.last_frame).as_secs_f64();

                if self.last_frame_times.len() == 120 {
                    self.last_frame_times.remove(1);
                }

                self.last_frame_times.push(delta);

                let fps: f64 =
                    self.last_frame_times.len() as f64 / self.last_frame_times.iter().sum::<f64>();
                imgui_io.set_delta_time(delta as f32);

                log::debug!("Frame delta {}s", delta);

                let ui = imgui_context.frame();

                ui.window("Hello")
                    .size([300.0, 120.0], dear_imgui_rs::Condition::FirstUseEver)
                    .build(|| {
                        ui.text("Hello, world!");
                        ui.text(format!("({:?} FPS)", fps));
                        if ui.button("Click me") {
                            println!("clicked");
                        }
                    });

                window
                    .redraw(Some(
                        |(texture, view): (&wgpu::SurfaceTexture, &wgpu::TextureView),
                         encoder: &mut wgpu::CommandEncoder| {
                            let mut imgui_renderer = state.imgui_renderer.lock().unwrap();

                            let frame =
                                imgui_context.render(imgui_renderer.renderer_consumer().unwrap());
                            let extent =
                                dear_imgui_wgpu::FramebufferExtent::from_texture(&texture.texture);

                            let mut render_pass =
                                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: None,
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                        view,
                                        depth_slice: None,
                                        resolve_target: None,
                                        ops: Default::default(),
                                    })],
                                    depth_stencil_attachment: None,
                                    timestamp_writes: None,
                                    occlusion_query_set: None,
                                    multiview_mask: None,
                                });

                            imgui_renderer
                                .render(frame, &mut render_pass, extent)
                                .unwrap();
                        },
                    ))
                    .unwrap();

                self.last_frame = now;

                window.window.request_redraw();
            }

            WindowEvent::Resized(size) => {
                window.resize(size.width, size.height).unwrap();

                window.window.request_redraw();
            }

            WindowEvent::Occluded(is_occluded) => {
                if !is_occluded {
                    window.window.request_redraw();
                }
            }

            WindowEvent::ScaleFactorChanged { .. } => {}
            WindowEvent::Focused(focused) => {
                imgui_io.add_focus_event(focused);
            }
            WindowEvent::Ime(_) => {}
            WindowEvent::CursorMoved {
                position: winit::dpi::PhysicalPosition { x, y },
                ..
            } => {
                imgui_io.add_mouse_pos_event([x as f32, y as f32]);
            }

            // WindowEvent::ActivationTokenDone { serial, token } => todo!(),
            // WindowEvent::Moved(physical_position) => todo!(),
            // WindowEvent::Destroyed => todo!(),
            // WindowEvent::DroppedFile(path_buf) => todo!(),
            // WindowEvent::HoveredFile(path_buf) => todo!(),
            // WindowEvent::HoveredFileCancelled => todo!(),
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => match event.logical_key {
                winit::keyboard::Key::Character(string) => {
                    imgui_io.add_input_characters_utf8(string.as_str());
                }
                winit::keyboard::Key::Named(named_key) => match winit_to_imgui_key(named_key) {
                    Some(key) => imgui_io.add_key_event(key, event.state.is_pressed()),
                    None => (),
                },
                _ => (),
            },
            // WindowEvent::ModifiersChanged(modifiers) => todo!(),
            // WindowEvent::CursorEntered { device_id } => todo!(),
            // WindowEvent::CursorLeft { device_id } => todo!(),
            WindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
            } => {
                let char_height = 12.0 * window.window.scale_factor() / 72.0; // 12pt -> px
                let (x, y) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        (x * (char_height / 2.0) as f32, y * char_height as f32)
                    }
                    winit::event::MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition {
                        x,
                        y,
                    }) => (x as f32, y as f32),
                };
                imgui_io.add_mouse_wheel_event([x, y]);
            }
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => {
                use winit::event::MouseButton::*;

                imgui_io.add_mouse_button_event(
                    match button {
                        Left => dear_imgui_rs::MouseButton::Left,
                        Right => dear_imgui_rs::MouseButton::Right,
                        Middle => dear_imgui_rs::MouseButton::Middle,
                        Forward => dear_imgui_rs::MouseButton::Extra1,
                        Back => dear_imgui_rs::MouseButton::Extra2,

                        _ => return,
                    },
                    state.is_pressed(),
                );
            }
            // WindowEvent::PinchGesture { device_id, delta, phase } => todo!(),
            // WindowEvent::PanGesture { device_id, delta, phase } => todo!(),
            // WindowEvent::DoubleTapGesture { device_id } => todo!(),
            // WindowEvent::RotationGesture { device_id, delta, phase } => todo!(),
            // WindowEvent::TouchpadPressure { device_id, pressure, stage } => todo!(),
            // WindowEvent::AxisMotion { device_id, axis, value } => todo!(),
            // WindowEvent::Touch(touch) => todo!(),
            // WindowEvent::ThemeChanged(theme) => todo!(),
            _ => {
                log::debug!("{event:?}");
            }
        }
    }
}

use dear_imgui_rs::Key as ImGuiKey;
use winit::keyboard::NamedKey;

fn winit_to_imgui_key(named_key: NamedKey) -> Option<ImGuiKey> {
    match named_key {
        NamedKey::Tab => Some(ImGuiKey::Tab),
        NamedKey::ArrowLeft => Some(ImGuiKey::LeftArrow),
        NamedKey::ArrowRight => Some(ImGuiKey::RightArrow),
        NamedKey::ArrowUp => Some(ImGuiKey::UpArrow),
        NamedKey::ArrowDown => Some(ImGuiKey::DownArrow),
        NamedKey::PageUp => Some(ImGuiKey::PageUp),
        NamedKey::PageDown => Some(ImGuiKey::PageDown),
        NamedKey::Home => Some(ImGuiKey::Home),
        NamedKey::End => Some(ImGuiKey::End),
        NamedKey::Insert => Some(ImGuiKey::Insert),
        NamedKey::Delete => Some(ImGuiKey::Delete),
        NamedKey::Backspace => Some(ImGuiKey::Backspace),
        NamedKey::Space => Some(ImGuiKey::Space),
        NamedKey::Enter => Some(ImGuiKey::Enter),
        NamedKey::Escape => Some(ImGuiKey::Escape),

        // Modifiers
        NamedKey::Control => Some(ImGuiKey::ModCtrl),
        NamedKey::Shift => Some(ImGuiKey::ModShift),
        NamedKey::Alt => Some(ImGuiKey::ModAlt),
        NamedKey::Super | NamedKey::Meta => Some(ImGuiKey::ModSuper),

        // Function keys
        NamedKey::F1 => Some(ImGuiKey::F1),
        NamedKey::F2 => Some(ImGuiKey::F2),
        NamedKey::F3 => Some(ImGuiKey::F3),
        NamedKey::F4 => Some(ImGuiKey::F4),
        NamedKey::F5 => Some(ImGuiKey::F5),
        NamedKey::F6 => Some(ImGuiKey::F6),
        NamedKey::F7 => Some(ImGuiKey::F7),
        NamedKey::F8 => Some(ImGuiKey::F8),
        NamedKey::F9 => Some(ImGuiKey::F10),
        NamedKey::F10 => Some(ImGuiKey::F10),
        NamedKey::F11 => Some(ImGuiKey::F11),
        NamedKey::F12 => Some(ImGuiKey::F12),

        // Fallback for unmapped or multimedia keys
        _ => None,
    }
}

fn main() {
    colog::default_builder()
        .filter_level(match std::env::var("PIRU_LOG_LEVEL") {
            Ok(level) => log::LevelFilter::from_str(&level).unwrap_or(log::LevelFilter::Info),
            Err(_) => log::LevelFilter::Info,
        })
        .init();

    Piru::new().run();
}
