use dear_imgui_rs::{StyleVar, WindowFlags};
use imgui_app::dear_imgui_rs::Condition;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let platform = imgui_app::platform_init("Don't forget to update the window title", (1280, 720))?;
    let renderer = imgui_app::renderer_init(&platform.window, platform.window.size())?;
    let imgui = imgui_app::imgui_init(platform, renderer);
    
    imgui_app::run(imgui, |ui, ex| {
        let (width, height) = ex.window.size();
        let _border_size = ui.push_style_var(StyleVar::WindowBorderSize(0.0));
        ui.window("Main")
            .position([0.0, 0.0], Condition::Always)
            .size([width as f32, height as f32], Condition::Always)
            .flags(WindowFlags::NO_MOVE | WindowFlags::NO_TITLE_BAR | WindowFlags::NO_RESIZE)
            .build(|| {
                ui.text("A B C D E F G H I J K L M N O P Q R S T U V W X Y Z");
                ui.text("a b c d e f g h i j k l m n o p q r s t u v w x y z");
                
                let _mono_font = ui.push_font(ex.fonts.mono);
                ui.text("A B C D E F G H I J K L M N O P Q R S T U V W X Y Z");
                ui.text("a b c d e f g h i j k l m n o p q r s t u v w x y z");
            });
        
        ui.show_demo_window(&mut true);
    });
    
    Ok(())
}
