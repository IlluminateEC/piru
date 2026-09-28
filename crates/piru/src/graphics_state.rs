use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use wgpu::{RenderPipeline, ShaderModule};

use crate::{error::RenderError, surface::Surface, window_manager::WindowManager};

pub type ShaderId = u32;
pub type PipelineId = u32;

pub struct ShaderRegistry {
    max_shaders: u32,
    max_pipelines: u32,

    shaders: HashMap<ShaderId, Arc<ShaderModule>>,
    render_pipelines: HashMap<PipelineId, Arc<RenderPipeline>>,
}

impl ShaderRegistry {
    pub fn add_shader(&mut self, module: ShaderModule) -> ShaderId {
        let id = self.max_shaders;
        self.max_shaders += 1;

        self.shaders.insert(id, Arc::new(module));

        id
    }

    pub fn add_pipeline(&mut self, vertex: ShaderId, fragment: ShaderId) -> PipelineId {
        let id = self.max_pipelines;
        self.max_pipelines += 1;

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: self.shaders.get(&vertex).unwrap(),
                entry_point: None,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
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
                module: self.shaders.get(&fragment).unwrap(),
                entry_point: None,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
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

        self.render_pipelines.insert(id, Arc::new(render_pipeline));

        id
    }
}

// TODO: really shouldn't be constant
const MSAA_SAMPLES: u32 = 4;

pub struct InitializedState {
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub swapchain_format: wgpu::TextureFormat,

    pub shader_registry: ShaderRegistry,
}

impl InitializedState {
    pub async fn new(
        graphics: Arc<GraphicsStateInternal>,
        surface: &Surface,
    ) -> Result<Self, RenderError> {
        let adapter = graphics
            .instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface.surface),
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

        let swapchain_format = surface.get_usable_swapchain_format(&adapter)?;

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

        let render_pipelines = vec![Arc::new(render_pipeline)];

        Ok(Self {
            adapter,
            device,
            queue,
            swapchain_format,

            shader_registry,
        })
    }
}

pub(crate) struct GraphicsStateInternal {
    pub instance: wgpu::Instance,
    state: Arc<RwLock<Option<InitializedState>>>,
}

impl GraphicsStateInternal {
    pub async fn initialize_if_not_initialized(
        self: &Arc<Self>,
        surface: &Surface,
    ) -> Result<(), RenderError> {
        if self
            .state
            .read()
            .map_err(|_| RenderError::GpuPoisoned)?
            .is_none()
        {
            *self.state.write().map_err(|_| RenderError::GpuPoisoned)? =
                Some(InitializedState::new(self.clone(), surface).await?);
        }

        Ok(())
    }

    pub fn with_state<R, RV>(
        &self,
        closure: impl FnOnce(&InitializedState) -> R,
    ) -> Result<RV, RenderError>
    where
        R: Into<Result<RV, RenderError>>,
    {
        let state = self.state.read().map_err(|_| RenderError::GpuPoisoned)?;
        let state = state.as_ref().ok_or(RenderError::NotInitializedYet)?;

        closure(&state).into()
    }
}

pub struct GraphicsState {
    pub graphics_state_internal: Arc<GraphicsStateInternal>,
    pub window_manager: WindowManager,
}

impl GraphicsState {
    pub fn new(display: Option<Box<dyn wgpu::wgt::WgpuHasDisplayHandle + 'static>>) -> Arc<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::default(),
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            display,
        });

        let graphics_state_internal = Arc::new(GraphicsStateInternal {
            instance,
            state: Arc::new(RwLock::new(None)),
        });

        Arc::new(Self {
            graphics_state_internal: graphics_state_internal.clone(),
            window_manager: WindowManager::new(graphics_state_internal),
        })
    }

    pub fn with_state<R, RV>(
        &self,
        closure: impl FnOnce(&InitializedState) -> R,
    ) -> Result<RV, RenderError>
    where
        R: Into<Result<RV, RenderError>>,
    {
        self.graphics_state_internal.with_state(closure)
    }

    pub async fn initialize_if_not_initialized(
        self: &Arc<Self>,
        surface: &Surface,
    ) -> Result<(), RenderError> {
        self.graphics_state_internal
            .initialize_if_not_initialized(surface)
            .await
    }
}
