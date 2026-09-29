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
        let wgpu_surface: wgpu::Surface<'_> = graphics_state
            .instance
            .create_surface(window.clone())
            .map_err(RenderError::CreateSurfaceError)?;

        let winit::dpi::PhysicalSize { width, height } = window.inner_size();

        graphics_state
            .initialize_if_not_initialized(&wgpu_surface)
            .await?;

        let mut configuration = graphics_state.with_state(|state| {
            wgpu_surface
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

        graphics_state.with_state(|state| {
            wgpu_surface.configure(&state.device, &configuration);

            Ok(())
        })?;

        let surface = Self {
            graphics_state: graphics_state.clone(),
            surface: wgpu_surface,
            configuration,
        };

        Ok(surface)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), RenderError> {
        self.configuration.width = width;
        self.configuration.height = height;

        self.graphics_state.with_state(|state| {
            self.surface.configure(&state.device, &self.configuration);
            Ok(())
        })?;

        Ok(())
    }

    pub fn get_view(&self) -> Option<(wgpu::SurfaceTexture, wgpu::TextureView)> {
        use wgpu::CurrentSurfaceTexture;

        let frame = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) | CurrentSurfaceTexture::Suboptimal(frame) => {
                frame
            }
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => {
                return None;
            }
            texture => todo!("{:?}", texture),
        };

        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.configuration.format),
            ..Default::default()
        });

        Some((frame, view))
    }

    pub fn redraw(&self) -> Result<(), RenderError> {
        self.graphics_state.with_state(|state| {
            let Some((frame, view)) = self.get_view() else {
                return Ok(());
            };

            let msaa_texture = state.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("MSAA Texture"),
                size: wgpu::Extent3d {
                    width: view.texture().width(),
                    height: view.texture().height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: crate::graphics_state::MSAA_SAMPLES,
                dimension: wgpu::TextureDimension::D2,
                format: frame.texture.format(),
                // format: gpu_state.swapchain_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let msaa_view = msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());

            let mut encoder = state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        // view: &view,
                        view: &msaa_view,
                        depth_slice: None,
                        // resolve_target: None,
                        resolve_target: Some(&view),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.25,
                                g: 0.0,
                                b: 0.25,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

                // TODO: actual pipelines
                render_pass.set_pipeline(state.shader_registry.get_pipeline(0).unwrap());

                // self.buffers.bind_to(&mut render_pass);

                render_pass.draw(0..18, 0..1);
            }

            state.queue.submit(Some(encoder.finish()));
            state.queue.present(frame);

            Ok(())
        })
    }

    pub fn get_usable_swapchain_format(
        surface: &wgpu::Surface<'_>,
        adapter: &wgpu::Adapter,
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
