use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};

use wgpu::{Device, RenderPipeline, ShaderModule};

use crate::{error::RenderError, window_manager::WindowManager};

pub type ShaderId = u32;

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct PipelineKey {
    pub vertex_shader_id: ShaderId,
    pub fragment_shader_id: ShaderId,
    pub format: wgpu::TextureFormat,
}

pub struct ShaderRegistry {
    device: Device,
    shaders: HashMap<ShaderId, ShaderModule>,
    next_id: ShaderId,
}

impl ShaderRegistry {
    pub fn new(device: Device) -> Self {
        Self {
            device,
            shaders: HashMap::new(),
            next_id: 0,
        }
    }

    #[allow(clippy::arithmetic_side_effects)]
    pub fn add(&mut self, module: ShaderModule) -> ShaderId {
        let id = self.next_id;
        self.next_id += 1;

        self.shaders.insert(id, module);

        id
    }

    pub fn get(&self, id: ShaderId) -> Option<&ShaderModule> {
        self.shaders.get(&id)
    }
}

// TODO: really shouldn't be constant
pub const MSAA_SAMPLES: u32 = 4;

pub struct PipelineCache {
    device: Device,
    pipelines: HashMap<PipelineKey, RenderPipeline>,
}

impl PipelineCache {
    pub fn new(device: Device) -> Self {
        Self {
            device,
            pipelines: HashMap::new(),
        }
    }

    pub fn get_pipeline(&self, key: PipelineKey) -> Option<&RenderPipeline> {
        self.pipelines.get(&key)
    }

    pub fn get_or_create_pipeline(
        &mut self,
        vertex_shader_id: ShaderId,
        fragment_shader_id: ShaderId,
        format: wgpu::TextureFormat,
        shaders: &ShaderRegistry,
    ) -> RenderPipeline {
        let key = PipelineKey {
            vertex_shader_id,
            fragment_shader_id,
            format,
        };

        if let Some(pipeline) = self.pipelines.get(&key) {
            return pipeline.clone();
        }

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                immediate_size: 0,
            });

        let render_pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: shaders
                        .get(vertex_shader_id)
                        .expect("Vertex shader not found"),
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
                    module: shaders
                        .get(fragment_shader_id)
                        .expect("Fragment shader not found"),
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,

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

        self.pipelines.insert(key, render_pipeline.clone());
        render_pipeline
    }
}

pub struct InitializedState {
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub shader_registry: ShaderRegistry,
    pub pipeline_cache: Mutex<PipelineCache>,
    // TODO: actual world entities w/ attached shaders
    pub vertex_shader_id: ShaderId,
    pub fragment_shader_id: ShaderId,
}

impl InitializedState {
    pub(crate) async fn new(
        graphics: Arc<GraphicsStateInternal>,
        surface: &wgpu::Surface<'_>,
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

        let mut shader_registry = ShaderRegistry::new(device.clone());

        let vertex_shader_id = shader_registry
            .add(device.create_shader_module(wgpu::include_spirv!("../../../shaders/vertex.spv")));
        let fragment_shader_id = shader_registry.add(
            device.create_shader_module(wgpu::include_spirv!("../../../shaders/fragment.spv")),
        );

        let pipeline_cache = Mutex::new(PipelineCache::new(device.clone()));

        Ok(Self {
            adapter,
            device,
            queue,
            shader_registry,
            pipeline_cache,
            vertex_shader_id,
            fragment_shader_id,
        })
    }
}

pub(crate) struct GraphicsStateInternal {
    pub instance: wgpu::Instance,
    pub state: Arc<RwLock<Option<InitializedState>>>,
}

impl GraphicsStateInternal {
    pub async fn initialize_if_not_initialized(
        self: &Arc<Self>,
        surface: &wgpu::Surface<'_>,
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

    pub fn with_state<R, RV, C>(&self, closure: C) -> Result<RV, RenderError>
    where
        C: FnOnce(&InitializedState) -> R,
        R: Into<Result<RV, RenderError>>,
    {
        let state = self.state.read().map_err(|_| RenderError::GpuPoisoned)?;
        let state = state.as_ref().ok_or(RenderError::NotInitializedYet)?;

        closure(state).into()
    }

    pub async fn with_state_async<R, RF, RV, C>(&self, closure: C) -> Result<RV, RenderError>
    where
        C: FnOnce(&InitializedState) -> RF,
        RF: Future<Output = R>,
        R: Into<Result<RV, RenderError>>,
    {
        let state = self.state.read().map_err(|_| RenderError::GpuPoisoned)?;
        let state = state.as_ref().ok_or(RenderError::NotInitializedYet)?;

        closure(state).await.into()
    }
}

pub struct GraphicsState {
    pub(crate) graphics_state_internal: Arc<GraphicsStateInternal>,
    pub window_manager: Mutex<WindowManager>,
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
            window_manager: Mutex::new(WindowManager::new(graphics_state_internal)),
        })
    }

    pub fn with_state<R, RV, C>(&self, closure: C) -> Result<RV, RenderError>
    where
        C: FnOnce(&InitializedState) -> R,
        R: Into<Result<RV, RenderError>>,
    {
        self.graphics_state_internal.with_state(closure)
    }

    pub async fn with_state_async<R, RF, RV, C>(&self, closure: C) -> Result<RV, RenderError>
    where
        C: FnOnce(&InitializedState) -> RF,
        RF: Future<Output = R>,
        R: Into<Result<RV, RenderError>>,
    {
        self.graphics_state_internal.with_state_async(closure).await
    }

    pub async fn initialize_if_not_initialized(
        self: &Arc<Self>,
        surface: &wgpu::Surface<'_>,
    ) -> Result<(), RenderError> {
        self.graphics_state_internal
            .initialize_if_not_initialized(surface)
            .await
    }
}
