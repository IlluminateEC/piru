use std::sync::Arc;

use crate::{GraphicsState, error::RenderError, graphics_state::GraphicsStateInternal};

pub struct Surface {
    graphics_state: Arc<GraphicsStateInternal>,
    pub surface: wgpu::Surface<'static>,
    configuration: wgpu::SurfaceConfiguration,
}

impl Surface {
    pub async fn new(
        graphics_state: Arc<GraphicsStateInternal>,
        window: Arc<winit::window::Window>,
    ) -> Result<Self, RenderError> {
        let surface = graphics_state
            .instance
            .create_surface(window.clone())
            .map_err(RenderError::CreateSurfaceError)?;

        graphics_state
            .initialize_if_not_initialized(&surface)
            .await?;

        let winit::dpi::PhysicalSize { width, height } = window.inner_size();

        let mut configuration = graphics_state.with_state(|state| {
            surface
                .get_default_config(&state.adapter, width, height)
                .ok_or_else(|| {
                    RenderError::UnsupportedHardwareNoError(
                        "Surface is unsupported by device".to_string(),
                    )
                })
        })?;

        configuration.format = graphics_state
            .state
            .read()
            .map_err(|_| RenderError::GpuPoisoned)?
            .as_ref()
            .ok_or(RenderError::NotInitializedYet)?
            .swapchain_format;

        Ok(Self {
            graphics_state,
            surface,
            configuration,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), RenderError> {
        self.configuration.width = width;
        self.configuration.height = height;

        self.graphics_state
            .with_state(|state| Ok(self.surface.configure(&state.device, &self.configuration)))?;

        Ok(())
    }

    pub fn redraw(&self) -> Result<(), RenderError> {
        self.graphics_state.with_state(|state| {
            let awawa = ();

            Ok(())
        })
    }

    pub fn get_usable_swapchain_format(
        &self,
        adapter: &wgpu::Adapter,
    ) -> Result<wgpu::TextureFormat, RenderError> {
        let swapchain_capabilities = self.surface.get_capabilities(adapter);

        [
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba8Unorm,
        ]
        .into_iter()
        .find(|format| swapchain_capabilities.formats.contains(format))
        .ok_or_else(|| RenderError::UnsupportedHardwareNoError("The device does not support either Bgra8Unorm or Rgba8Unorm but at least one is required.".to_string()))
    }
}
