use dear_imgui_rs::{StyleVar, WindowFlags};
use imgui_app::dear_imgui_rs::Condition;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let platform = imgui_app::platform_init("Don't forget to update the window title", (1280, 720))?;
    let renderer = imgui_app::renderer_init(&platform.window, platform.window.size())?;
    let imgui = imgui_app::imgui_init(platform, renderer);
    
    let texture_bytes = make_image(64, 64);
    let mut texture_id = None::<u64>;
    
    imgui_app::run(imgui, |ui, mut ex| {
        if texture_id.is_none() {
            let id = ex.textures.create_texture_from_bytes(64, 64, &texture_bytes);
            texture_id = Some(id);
        }
        
        let (width, height) = ex.window.size();
        let _border_size = ui.push_style_var(StyleVar::WindowBorderSize(0.0));
        ui.window("Main")
            .position([0.0, 0.0], Condition::Always)
            .size([width as f32, height as f32], Condition::Always)
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
                let texture = ex.textures.get_texture(texture_id.unwrap()).unwrap();
                ui.image(texture, texture.size());
            });
        
        ui.show_demo_window(&mut true);
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
