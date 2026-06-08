pub struct RendererState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub backend: Option<dear_imgui_wgpu::WgpuRenderer>,
}

#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("Failed to get display or window handle.")]
    Handle(#[from] raw_window_handle::HandleError),
    #[error("Failed to create surface.")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("Failed to obtain adapter.")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),
    #[error("Failed to obtain device.")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
}

pub fn renderer_init<W>(window: &W, size: (u32, u32)) -> Result<RendererState, RendererError>
where
    W: raw_window_handle::HasDisplayHandle + raw_window_handle::HasWindowHandle
{
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let surface = unsafe {
        let surface_target = wgpu::SurfaceTargetUnsafe::from_display_and_window(window, window)?;
        instance.create_surface_unsafe(surface_target)?
    };
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    }))?;
    
    let (device, queue) = pollster::block_on(
        adapter.request_device(&wgpu::DeviceDescriptor::default())
    )?;

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);
    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.0,
        height: size.1,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: Vec::default(),
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &surface_config);
    
    Ok(RendererState {
        device,
        queue,
        surface,
        surface_config,
        backend: None,
    })
}
