pub mod error;
pub mod graphics_state;
pub mod surface;
pub mod window;
pub mod window_manager;

use std::{
    collections::HashMap,
    hint::unreachable_unchecked,
    sync::Arc,
    task::{Context, Waker},
};

use wgpu::{PipelineCompilationOptions, ShaderModule, Surface, TextureFormat};
use winit::{application::ApplicationHandler, window::WindowId};

use crate::{
    error::RenderError, graphics_state::GraphicsState, window::Window,
    window_manager::WindowManager,
};

pub struct InitializedState {
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub swapchain_format: TextureFormat,

    pub shaders: HashMap<String, Arc<ShaderModule>>,
    pub render_pipelines: Vec<Arc<wgpu::RenderPipeline>>,
}

const MSAA_SAMPLES: u32 = 4;

impl InitializedState {
    pub async fn new(
        graphics: Arc<GraphicsState>,
        surface: &Surface<'_>,
    ) -> Result<Self, RenderError> {
        let adapter = graphics
            .instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(surface),
                apply_limit_buckets: false,
            })
            .await
            .map_err(|error| RenderError::UnsupportedHardware {
                message: "Unable to find an adapter that supports Piru.".to_string(),
                error: Box::new(error),
            })?;

        let (device, queue) = adapter
            .request_device(&wgpu::wgt::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::default(),
                required_limits: wgpu::Limits::defaults().using_resolution(adapter.limits()),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|error| RenderError::UnsupportedHardware {
                message: "Unable to find a device that supports Piru.".to_string(),
                error: Box::new(error),
            })?;

        log::info!(
            "Using rendering device: {}\n\tBackend: {}\n\tType: {:?}\n\tDriver: {}\n\tDriver info: {}",
            device.adapter_info().name,
            device.adapter_info().backend,
            device.adapter_info().device_type,
            device.adapter_info().driver,
            device.adapter_info().driver_info,
        );

        let swapchain_format = Self::get_usable_swapchain_format(&adapter, surface)?;

        let mut shaders = HashMap::new();

        shaders.insert(
            "vertex".to_string(),
            Arc::new(
                device.create_shader_module(wgpu::include_spirv!("../../../shaders/vertex.spv")),
            ),
        );
        shaders.insert(
            "fragment".to_string(),
            Arc::new(
                device.create_shader_module(wgpu::include_spirv!("../../../shaders/fragment.spv")),
            ),
        );

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shaders.get("vertex").unwrap(),
                entry_point: None,
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: MSAA_SAMPLES,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: shaders.get("fragment").unwrap(),
                entry_point: None,
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: swapchain_format,

                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),

                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let render_pipelines = vec![Arc::new(render_pipeline)];

        Ok(Self {
            adapter,
            device,
            queue,
            swapchain_format,
            shaders,
            render_pipelines,
        })
    }

    fn get_usable_swapchain_format(
        adapter: &wgpu::Adapter,
        surface: &wgpu::Surface,
    ) -> Result<wgpu::TextureFormat, RenderError> {
        let swapchain_capabilities = surface.get_capabilities(adapter);

        [
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba8Unorm,
        ]
        .into_iter()
        .find(|format| swapchain_capabilities.formats.contains(format))
        .ok_or_else(|| RenderError::UnsupportedHardwareNoError("The device does not support either Bgra8Unorm or Rgba8Unorm but at least one is required.".to_string()))
    }
}

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
                spin_until_ready(state.window_manager.create_window(event_loop)).unwrap();
            }
            None => unsafe { unreachable_unchecked() },
        }
    }

    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.graphics_state = None;
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        // match event {}
    }
}

fn spin_until_ready<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());

    loop {
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(value) => return value,
            std::task::Poll::Pending => (),
        }
    }
}

fn main() {
    colog::default_builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    Piru::new().run();
}
