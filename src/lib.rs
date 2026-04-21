mod textures;
mod png_decoder;
mod platform;
mod renderer;
mod extensions;

use std::time::Instant;

pub use platform::{PlatformState, platform_init};
pub use renderer::{RendererState, renderer_init};
pub use textures::{Textures, Texture};
pub use extensions::ImguiCursorExt;

pub use imgui;
pub use sdl2;
pub use imgui_sdl2_support;
pub use wgpu;
pub use imgui_wgpu;

const IDLE_FPS: f64 = 60.0;
const IDLE_FRAME_DURATION_MS: f64 = 1000.0 / IDLE_FPS;

pub struct ImguiState {
    pub platform: PlatformState,
    pub renderer: RendererState,
    pub context: imgui::Context,
    pub fonts: Fonts,
}

pub struct Fonts {
    pub ui: imgui::FontId,
    pub mono: imgui::FontId,
}

pub struct Extras<'a> {
    pub window: &'a sdl2::video::Window,
    pub fonts: &'a Fonts,
    pub textures: &'a mut Textures<'a>,
}

pub fn imgui_init(mut platform: PlatformState, mut renderer: RendererState) -> ImguiState {
    let mut context = imgui::Context::create();
    context.set_ini_filename(None);
    context.set_log_filename(None);
    
    // Use FreeType for font rasterization
    unsafe {
        use imgui::internal::RawCast;
        context.fonts().raw_mut().FontBuilderIO = imgui::sys::ImGuiFreeType_GetBuilderForFreeType();
    }
    
    let ui_font = context.fonts().add_font(&[
        imgui::FontSource::TtfData {
            data: include_bytes!("../resources/FiraSans-Regular.ttf"),
            size_pixels: 19.0,
            config: None,
        }
    ]);
    
    let mono_font = context.fonts().add_font(&[
        imgui::FontSource::TtfData {
            data: include_bytes!("../resources/FiraCode-Regular.ttf"),
            size_pixels: 19.0,
            config: None,
        }
    ]);
    
    let fonts = Fonts {
        ui: ui_font,
        mono: mono_font,
    };
    
    let imgui_platform = imgui_sdl2_support::SdlPlatform::new(&mut context);
    platform.im_platform = Some(imgui_platform);

    let renderer_config = imgui_wgpu::RendererConfig {
        texture_format: renderer.surface_config.format,
        ..Default::default()
    };
    let imgui_renderer = imgui_wgpu::Renderer::new(&mut context, &renderer.device, &renderer.queue, renderer_config);
    renderer.im_renderer = Some(imgui_renderer);

    ImguiState {
        context,
        platform,
        renderer,
        fonts,
    }
}

pub fn run<F>(imgui: ImguiState, mut build: F) where
    F: FnMut(&imgui::Ui, Extras)
{
    use sdl2::event::{Event, WindowEvent};
    
    let ImguiState { mut platform, mut renderer, context: mut im_context, fonts } = imgui;
    let event_pump = &mut platform.event_pump;
    let window = &mut platform.window;
    let im_platform = platform.im_platform.as_mut().expect("Platform should be initialized");
    let im_renderer = renderer.im_renderer.as_mut().expect("Renderer should be initialized");
    
    let mut last_frame_start = Instant::now();
    let mut is_in_background = false;
    
    // ui.push_style_color() doesn't work
    im_context.style_mut().colors[imgui::StyleColor::ModalWindowDimBg as usize] = [0.0, 0.0, 0.0, 0.95];
    
    'main_loop: loop {
        // Calculate how long to wait for the next event without missing fps target
        let last_frame_duration_ms = (Instant::now() - last_frame_start).as_secs_f64() * 1000.0;
        let wait_ms = f64::max(0.0, f64::round(IDLE_FRAME_DURATION_MS - last_frame_duration_ms)) as u32;
        let first_event = event_pump.wait_event_timeout(wait_ms);
        last_frame_start = Instant::now();
        
        // Process events
        if let Some(first_event) = first_event {
            let events = [first_event]
                .into_iter()
                .chain(event_pump.poll_iter());
            for event in events {
                im_platform.handle_event(&mut im_context, &event);
                
                if let Event::Window { window_id, .. } = event
                    && window_id != window.id()
                {
                    continue;
                }
                
                match event {
                    Event::Window { win_event: WindowEvent::SizeChanged(width, height), .. } => {
                        renderer.surface_config.width = width as u32;
                        renderer.surface_config.height = height as u32;
                        renderer.surface.configure(&renderer.device, &renderer.surface_config);
                    }
                    Event::Window { win_event: WindowEvent::Minimized, .. } => {
                        is_in_background = true;
                    }
                    Event::Window { win_event: WindowEvent::Exposed, .. } => {
                        is_in_background = false;
                        renderer.surface.configure(&renderer.device, &renderer.surface_config);
                    }
                    Event::Quit { .. } => break 'main_loop,
                    _ => {},
                }
            }
        }
        
        if is_in_background {
            continue;
        }
        
        // Prepare for new frame
        im_platform.prepare_frame(&mut im_context, window, event_pump);
        let ui = im_context.new_frame();
        
        // Build UI
        {
            let mut textures = Textures {
                renderer: im_renderer,
                device: &renderer.device,
                queue: &renderer.queue,
            };
            let extras = Extras {
                window,
                fonts: &fonts,
                textures: &mut textures,
            };
            build(ui, extras);
        }
        
        // Prepare to render
        let draw_data = im_context.render();
        let frame = match renderer.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(_)
                | wgpu::CurrentSurfaceTexture::Outdated
            => {
                renderer.surface.configure(&renderer.device, &renderer.surface_config);
                continue 'main_loop;
            }
            wgpu::CurrentSurfaceTexture::Timeout
                | wgpu::CurrentSurfaceTexture::Occluded
            => {
                continue 'main_loop;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                eprintln!("Warning: a validation error occurred while getting the current surface texture.");
                continue 'main_loop;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                eprintln!("Critical error: the window surface was lost.");
                break 'main_loop;
            }
        };
        let render_target = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = renderer.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("command encoder"),
        });

        // Render imgui
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &render_target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            im_renderer.render(draw_data, &renderer.queue, &renderer.device, &mut render_pass)
                .expect("Failed to render");
        }

        // Submit commands and present next frame
        renderer.queue.submit([encoder.finish()]);
        frame.present();
    }
}
