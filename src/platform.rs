#[allow(dead_code)]
pub struct PlatformState {
    pub sdl: sdl2::Sdl,
    pub video_subsystem: sdl2::VideoSubsystem,
    pub window: sdl2::video::Window,
    pub event_pump: sdl2::EventPump,
    pub im_platform: Option<imgui_sdl2_support::SdlPlatform>,
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("Failed to initialize SDL2: {0}")]
    SdlInitFailed(String),
    #[error("Failed to initialize video subsystem: {0}")]
    VideoInitFailed(String),
    #[error("Failed to create window: {0}")]
    WindowCreationFailed(String),
    #[error("The window's title was invalid because it contained a NUL byte.")]
    NulInWindowTitle(std::ffi::NulError),
    #[error("The window's width was too large ({0}).")]
    WindowWidthTooLarge(u32),
    #[error("The window's height was too large ({0}).")]
    WindowHeightTooLarge(u32),
    #[error("Failed to obtain event pump: {0}")]
    EventPump(String),
}

pub fn platform_init(title: &str, size: (u32, u32)) -> Result<PlatformState, PlatformError> {
    use sdl2::video::WindowBuildError;
    
    let sdl = sdl2::init().map_err(PlatformError::SdlInitFailed)?;
    let video_subsystem = sdl.video().map_err(PlatformError::VideoInitFailed)?;
    let window = video_subsystem.window(title, size.0, size.1)
        .position_centered()
        .resizable()
        .metal_view()
        .build()
        .map_err(|err| match err {
            WindowBuildError::SdlError(msg) => PlatformError::WindowCreationFailed(msg),
            WindowBuildError::WidthOverflows(width) => PlatformError::WindowWidthTooLarge(width),
            WindowBuildError::HeightOverflows(height) => PlatformError::WindowHeightTooLarge(height),
            WindowBuildError::InvalidTitle(err) => PlatformError::NulInWindowTitle(err),
        })?;
    let event_pump = sdl.event_pump().map_err(PlatformError::EventPump)?;
    sdl2::hint::set("SDL_MOUSE_FOCUS_CLICKTHROUGH", "1");
    
    Ok(PlatformState {
        sdl,
        video_subsystem,
        window,
        event_pump,
        im_platform: None,
    })
}
