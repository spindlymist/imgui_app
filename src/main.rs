use imgui_app::imgui::Condition;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let platform = imgui_app::platform_init("Don't forget to update the window title", (1024, 768))?;
    let renderer = imgui_app::renderer_init(&platform.window, platform.window.size())?;
    let imgui = imgui_app::imgui_init(platform, renderer);
    
    imgui_app::run(imgui, |ui, ex| {
        let (width, height) = ex.window.size();
        let _window = ui.window("Main")
            .position([0.0, 0.0], Condition::Always)
            .size([width as f32, height as f32], Condition::Always)
            .title_bar(false)
            .movable(false)
            .resizable(false)
            .begin();
        
        ui.text("Hello");
    });
    
    Ok(())
}
