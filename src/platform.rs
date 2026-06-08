#[allow(dead_code)]
pub struct PlatformState {
    pub sdl: sdl3::Sdl,
    pub video_subsystem: sdl3::VideoSubsystem,
    pub window: sdl3::video::Window,
    pub event_pump: sdl3::EventPump,
    pub scale: f32,
    pub backend: Option<dear_imgui_sdl3::Sdl3PlatformBackend>,
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("Failed to initialize SDL3: {0}")]
    SdlInitFailed(sdl3::Error),
    #[error("Failed to initialize video subsystem: {0}")]
    VideoInitFailed(sdl3::Error),
    #[error("Failed to create window: {0}")]
    WindowCreationFailed(sdl3::Error),
    #[error("The window's title was invalid because it contained a NUL byte.")]
    NulInWindowTitle(std::ffi::NulError),
    #[error("The window's width was too large ({0}).")]
    WindowWidthTooLarge(u32),
    #[error("The window's height was too large ({0}).")]
    WindowHeightTooLarge(u32),
    #[error("Failed to obtain event pump: {0}")]
    EventPump(sdl3::Error),
}

pub fn platform_init(title: &str, (width, height): (u32, u32)) -> Result<PlatformState, PlatformError> {
    use sdl3::video::WindowBuildError;
    
    dear_imgui_sdl3::enable_native_ime_ui();
    
    let sdl = sdl3::init().map_err(PlatformError::SdlInitFailed)?;
    let video_subsystem = sdl.video().map_err(PlatformError::VideoInitFailed)?;
    
    let scale = video_subsystem.get_primary_display()
        .and_then(|display| display.get_content_scale())
        .unwrap_or(1.0);
    let width_scaled = (scale * width as f32) as u32;
    let height_scaled = (scale * height as f32) as u32;
    
    let window = video_subsystem.window(title, width_scaled, height_scaled)
        .position_centered()
        .resizable()
        .metal_view()
        .high_pixel_density()
        .build()
        .map_err(|err| match err {
            WindowBuildError::SdlError(msg) => PlatformError::WindowCreationFailed(msg),
            WindowBuildError::WidthOverflows(width) => PlatformError::WindowWidthTooLarge(width),
            WindowBuildError::HeightOverflows(height) => PlatformError::WindowHeightTooLarge(height),
            WindowBuildError::InvalidTitle(err) => PlatformError::NulInWindowTitle(err),
        })?;
    let event_pump = sdl.event_pump().map_err(PlatformError::EventPump)?;
    sdl3::hint::set("SDL_MOUSE_FOCUS_CLICKTHROUGH", "1");
    
    Ok(PlatformState {
        sdl,
        video_subsystem,
        window,
        event_pump,
        scale,
        backend: None,
    })
}
