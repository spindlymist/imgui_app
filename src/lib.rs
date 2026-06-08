mod textures;
mod png_decoder;
mod platform;
mod renderer;
mod extensions;

use std::time::{Duration, Instant};

pub use platform::{PlatformState, platform_init};
pub use renderer::{RendererState, renderer_init};
pub use textures::{Textures, Texture};
pub use extensions::ImguiCursorExt;

pub use dear_imgui_rs;
pub use dear_imgui_sdl3;
pub use dear_imgui_wgpu;
pub use sdl3;
pub use wgpu;

const IDLE_FPS: f64 = 60.0;
const IDLE_FRAME_DURATION_MS: f64 = 1000.0 / IDLE_FPS;

pub struct ImguiState {
    pub platform: PlatformState,
    pub renderer: RendererState,
    pub imgui: dear_imgui_rs::Context,
    pub fonts: Fonts,
}

pub struct Fonts {
    pub ui: dear_imgui_rs::FontId,
    pub mono: dear_imgui_rs::FontId,
}

pub struct Extras<'a> {
    pub window: &'a sdl3::video::Window,
    pub fonts: &'a Fonts,
    pub textures: &'a mut Textures,
}

pub fn imgui_init(mut platform: PlatformState, mut renderer: RendererState) -> ImguiState {
    let mut imgui = dear_imgui_rs::Context::create();
    
    // Configure imgui
    {
        let mut flags = imgui.io_mut().config_flags();
        flags |= dear_imgui_rs::ConfigFlags::NAV_ENABLE_KEYBOARD;
        imgui.io_mut().set_config_flags(flags);
        
        let _ = imgui.set_ini_filename(None::<String>);
        let _ = imgui.set_log_filename(None::<String>);
        
        imgui.style_mut().set_font_scale_dpi(platform.scale);
        imgui.style_mut().set_color(dear_imgui_rs::StyleColor::ModalWindowDimBg, [0.0, 0.0, 0.0, 0.95]);
    }
    
    // Create the platform backend
    {
        let platform_backend = dear_imgui_sdl3::Sdl3PlatformBackend::init_for_vulkan(
            &mut imgui,
            &platform.window,
        ).unwrap();
        platform.backend = Some(platform_backend);
        
        let flags = imgui.io_mut().backend_flags();
        imgui.io_mut().set_backend_flags(flags);
    }

    // Create the rendering backend
    {
        let renderer_info = dear_imgui_wgpu::WgpuInitInfo::new(
            renderer.device.clone(),
            renderer.queue.clone(),
            renderer.surface_config.format,
        );
        let mut render_backend = dear_imgui_wgpu::WgpuRenderer::new(renderer_info, &mut imgui).unwrap();
        render_backend.set_gamma_mode(dear_imgui_wgpu::GammaMode::Auto);
        renderer.backend = Some(render_backend);
    }

    let fonts = create_fonts(&mut imgui);
    
    ImguiState {
        imgui,
        platform,
        renderer,
        fonts,
    }
}

pub fn run<F>(imgui: ImguiState, mut build: F) where
    F: FnMut(&dear_imgui_rs::Ui, Extras)
{
    use sdl3::event::{Event, WindowEvent};
    
    let ImguiState { mut platform, mut renderer, mut imgui, fonts } = imgui;
    let event_pump = &mut platform.event_pump;
    let window = &mut platform.window;
    let mut platform_backend = platform.backend.take().expect("Platform should be initialized");
    let mut render_backend = renderer.backend.take().expect("Renderer should be initialized");
    let mut textures = Textures::new(&renderer.device);
    
    let mut is_in_background = false;
    let mut last_frame_start = Instant::now();
    
    'main_loop: loop {
        // Calculate how long to wait for the next event without missing fps target
        let last_frame_duration_s = (Instant::now() - last_frame_start).as_secs_f64();
        let last_frame_duration_ms = last_frame_duration_s * 1000.0;
        let wait_ms = f64::max(0.0, f64::round(IDLE_FRAME_DURATION_MS - last_frame_duration_ms)) as u64;
        let first_event = event_pump.wait_event_timeout(Duration::from_millis(wait_ms));
        last_frame_start = Instant::now();
        
        // Process events
        if let Some(first_event) = first_event {
            let events = [first_event]
                .into_iter()
                .chain(event_pump.poll_iter());
            for event in events {
                if let Some(event_ll) = event.to_ll() {
                    platform_backend.process_event(&mut imgui, &event_ll);
                }
                
                if let Event::Window { window_id, .. } = event
                    && window_id != window.id()
                {
                    continue;
                }
                
                match event {
                    Event::Window { win_event: WindowEvent::Resized(width, height), .. } => {
                        if width > 0 && height > 0 {
                            renderer.surface_config.width = width as u32;
                            renderer.surface_config.height = height as u32;
                            renderer.surface.configure(&renderer.device, &renderer.surface_config);
                            println!("Resized surface: {}x{}", renderer.surface_config.width,
                                renderer.surface_config.height);
                        }
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
        
        imgui.io_mut().set_delta_time(last_frame_duration_s as f32);
        imgui.io_mut().set_display_size([
            renderer.surface_config.width as f32 * platform.scale,
            renderer.surface_config.height as f32 * platform.scale,
        ]);
        platform_backend.new_frame(&mut imgui);
            
        // Build the UI
        {
            let ui = imgui.frame();
            let extras = Extras {
                window,
                fonts: &fonts,
                textures: &mut textures,
            };
            build(ui, extras);
        }
        
        // Register new textures
        textures.register_textures(&mut imgui);
        
        // Prepare to render the next frame
        let (frame, reconfigure_after_present) = match renderer.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                renderer.surface.configure(&renderer.device, &renderer.surface_config);
                continue 'main_loop;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                continue 'main_loop;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                eprintln!("Warning: a validation error occurred while getting the current surface texture.");
                continue 'main_loop;
            }
        };
        let render_target = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = renderer.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // Render pass: render imgui
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
            
            let draw_data = imgui.render();
            match render_backend.new_frame() {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("Failed to render: {err}");
                    continue 'main_loop;
                }
            }
            match render_backend.render_draw_data_with_fb_size(
                draw_data,
                &mut render_pass,
                renderer.surface_config.width,
                renderer.surface_config.height,
            ) {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("Failed to render: {err}");
                    continue 'main_loop;
                }
            }
        }

        // Submit commands and present next frame
        renderer.queue.submit([encoder.finish()]);
        frame.present();
        
        if reconfigure_after_present {
            renderer.surface.configure(&renderer.device, &renderer.surface_config);
        }
    }
    
    render_backend.shutdown();
    platform_backend.shutdown(&mut imgui);
}

fn create_fonts(imgui: &mut dear_imgui_rs::Context) -> Fonts {
    let mut fonts = imgui.font_atlas_mut();
    
    // unsafe {
    //     let atlas = fonts.raw();
    //     let loader = dear_imgui_sys::ImGuiFreeType_GetFontLoader();
    //     dear_imgui_rs::sys::ImFontAtlas_SetFontLoader(atlas, loader);
    // }
    
    let ui_font = fonts.add_font(&[
        dear_imgui_rs::FontSource::TtfData {
            data: include_bytes!("../resources/FiraSans-Regular.ttf"),
            size_pixels: Some(19.0),
            config: None,
        }
    ]);
    
    let mono_font = fonts.add_font(&[
        dear_imgui_rs::FontSource::TtfData {
            data: include_bytes!("../resources/FiraCode-Regular.ttf"),
            size_pixels: Some(19.0),
            config: None,
        }
    ]);
    
    Fonts {
        ui: ui_font,
        mono: mono_font,
    }
}
