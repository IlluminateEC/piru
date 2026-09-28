#[derive(Debug)]
pub enum RenderError {
    WindowError(winit::error::OsError),
    CreateSurfaceError(wgpu::CreateSurfaceError),

    UnsupportedHardware {
        message: String,
        error: Box<dyn std::error::Error>,
    },
    UnsupportedHardwareNoError(String),

    GpuPoisoned,
    NotInitializedYet,
}
