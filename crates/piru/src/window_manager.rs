use std::{collections::HashMap, sync::Arc};

use crate::{error::RenderError, graphics_state::GraphicsStateInternal, window::Window};

pub type WindowId = winit::window::WindowId;

pub struct WindowManager {
    graphics_state: Arc<GraphicsStateInternal>,
    windows: HashMap<WindowId, Arc<Window>>,
}

impl WindowManager {
    pub(crate) fn new(graphics_state: Arc<GraphicsStateInternal>) -> Self {
        Self {
            graphics_state,
            windows: HashMap::new(),
        }
    }

    pub fn get_window(&self, id: WindowId) -> Option<&Arc<Window>> {
        self.windows.get(&id)
    }

    // Guh. Can't really fix it, so…
    #[allow(clippy::future_not_send)]
    pub async fn create_window(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<(WindowId, Arc<Window>), RenderError> {
        let window = Window::new(self.graphics_state.clone(), event_loop).await?;

        window.window.set_title(":3");
        window.window.set_visible(true);

        let id = window.window.id();

        self.windows.insert(id, window.clone());

        Ok((id, window))
    }

    pub fn remove_window(&mut self, id: WindowId) -> Result<(), RenderError> {
        self.windows.remove(&id);

        Ok(())
    }

    pub fn all_windows_closed(&self) -> bool {
        self.windows.is_empty()
    }
}
