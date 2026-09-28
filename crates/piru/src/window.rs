use std::sync::Arc;

use crate::{
    GraphicsState, error::RenderError, graphics_state::GraphicsStateInternal, surface::Surface,
};

pub struct Window {
    graphics_state: Arc<GraphicsStateInternal>,
    pub window: Arc<winit::window::Window>,
    pub surface: Surface,

    pub is_occluded: bool,
}

impl Window {
    pub async fn new(
        graphics_state: Arc<GraphicsStateInternal>,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<Arc<Self>, RenderError> {
        let window = Arc::new(
            event_loop
                .create_window(winit::window::WindowAttributes::default())
                .map_err(RenderError::WindowError)?,
        );

        let surface = Surface::new(graphics_state.clone(), window.clone()).await?;

        Ok(Arc::new(Self {
            graphics_state,
            window,
            surface,
            is_occluded: false,
        }))
    }

    pub const fn set_occluded(&mut self, occluded: bool) {
        self.is_occluded = occluded;
    }
}
