use dear_imgui_rs::{StyleVar, TextureId, WindowFlags};
use imgui_app::{Task, dear_imgui_rs::Condition};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let platform = imgui_app::platform_init("Don't forget to update the window title", (1280, 720))?;
    let renderer = imgui_app::renderer_init(&platform.window, platform.window.size())?;
    let imgui = imgui_app::imgui_init(platform, renderer);
    
    let texture_bytes = make_image(64, 64);
    let mut texture_id = None::<TextureId>;
    
    let mut text = String::new();
    
    imgui_app::run(imgui, |ui, mut ex| {
        if texture_id.is_none() {
            let id = ex.textures.create_texture(64, 64, &texture_bytes);
            texture_id = Some(id);
        }
        
        let _border_size = ui.push_style_var(StyleVar::WindowBorderSize(0.0));
        
        if let Some(_menu) = ui.begin_main_menu_bar() {
            if let Some(_file_menu) = ui.begin_menu("File") {
                if ui.menu_item("Exit") {
                    return Task::Exit;
                }
            }
        }
        
        let (viewport_width, viewport_height) = ex.window.size();
        let menu_bar_height = ui.current_font_size() + 2.0 * unsafe { ui.style().frame_padding()[1] };
        let window_width = viewport_width as f32;
        let window_height = viewport_height as f32 - menu_bar_height;
        ui.window("Main")
            .position([0.0, menu_bar_height], Condition::Always)
            .size([window_width, window_height], Condition::Always)
            .flags(WindowFlags::NO_MOVE | WindowFlags::NO_TITLE_BAR | WindowFlags::NO_RESIZE)
            .build(|| {
                ui.text("A B C D E F G H I J K L M N O P Q R S T U V W X Y Z");
                ui.text("a b c d e f g h i j k l m n o p q r s t u v w x y z");
                ui.new_line();
                
                ui.text("Now with symbols ⓘ ⛾⛿☯☸⛩⛰⛱⛴⛷⛸♸⚥☊☍☓☤🄰🄱🆈🆉⚖♇♪♬");
                ui.new_line();
                
                let _mono_font = ui.push_font(ex.fonts.mono);
                ui.text("A B C D E F G H I J K L M N O P Q R S T U V W X Y Z");
                ui.text("a b c d e f g h i j k l m n o p q r s t u v w x y z");
                
                ui.new_line();
                let texture_info = ex.textures.get_texture_info(texture_id.unwrap()).unwrap();
                ui.get_window_draw_list().set_sampler_nearest();
                ui.image(texture_info, [texture_info.width() * 4.0, texture_info.height() * 4.0]);
                ui.same_line();
                ui.get_window_draw_list().set_sampler_linear();
                ui.image(texture_info, [texture_info.width() * 4.0, texture_info.height() * 4.0]);
                
                ui.input_text("##InputText", &mut text).build();
                ui.same_line();
                if ui.button("Copy") {
                    let _ = ex.clipboard.set_clipboard_text(&text);
                }
                ui.same_line();
                if ui.button("Paste") {
                    if let Ok(clipboard_text) = ex.clipboard.clipboard_text() {
                        text = clipboard_text;
                    }
                }
            });
        
        Task::None
    });
    
    Ok(())
}

fn make_image(width: usize, height: usize) -> Vec<u8> {
    let n_bytes = width * height * 4;
    let mut bytes = Vec::with_capacity(n_bytes);
    bytes.resize(n_bytes, 0);
    
    let mut i = 0;
    for y in 0..64 {
        for x in 0..64 {
            let x_proportion = x as f32 / 63.0;
            let y_proportion = y as f32 / 63.0;
            bytes[i + 0] = 0;
            bytes[i + 1] = (x_proportion * 255.0).round() as u8;
            bytes[i + 2] = (y_proportion * 255.0).round() as u8;
            bytes[i + 3] = 255;
            i += 4;
        }
    }
    
    bytes
}
