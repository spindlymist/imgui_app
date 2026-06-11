use dear_imgui_rs::Ui;

pub trait ImguiExt {
    fn calc_text_size(&self, text: &str) -> [f32; 2];
    fn align_next_item_center(&self, width: f32);
    fn align_next_item_right(&self, width: f32);
}

impl ImguiExt for Ui {
    fn calc_text_size(&self, text: &str) -> [f32; 2] {
        self.current_font().calc_text_size(
            self.current_font_size(),
            f32::MAX,
            0.0,
            text
        )
    }
    
    fn align_next_item_center(&self, item_width: f32) {
        let width_avail = self.get_content_region_avail()[0];
        if width_avail > item_width {
            let delta_x = f32::round((width_avail - item_width) / 2.0);
            self.move_cursor_right(delta_x);
        }
    }
    
    fn align_next_item_right(&self, item_width: f32) {
        let width_avail = self.get_content_region_avail()[0];
        if width_avail > item_width {
            let delta_x = f32::floor(width_avail - item_width);
            self.move_cursor_right(delta_x);
        }
    }
}

pub trait ImguiCursorExt {
    fn move_cursor(&self, delta: [f32; 2]);
    fn move_cursor_up(&self, delta: f32);
    fn move_cursor_down(&self, delta: f32);
    fn move_cursor_left(&self, delta: f32);
    fn move_cursor_right(&self, delta: f32);
    fn set_cursor_x(&self, x: f32);
    fn set_cursor_y(&self, y: f32);
    fn cursor_x(&self) -> f32;
    fn cursor_y(&self) -> f32;
}

impl ImguiCursorExt for Ui {
    fn move_cursor(&self, delta: [f32; 2]) {
        let mut pos = self.cursor_pos();
        pos[0] += delta[0];
        pos[1] += delta[1];
        self.set_cursor_pos(pos);
    }
    
    fn move_cursor_up(&self, delta: f32) {
        let mut pos = self.cursor_pos();
        pos[1] -= delta;
        self.set_cursor_pos(pos);
    }

    fn move_cursor_down(&self, delta: f32) {
        let mut pos = self.cursor_pos();
        pos[1] += delta;
        self.set_cursor_pos(pos);
    }
    
    fn move_cursor_left(&self, delta: f32) {
        let mut pos = self.cursor_pos();
        pos[0] -= delta;
        self.set_cursor_pos(pos);
    }

    fn move_cursor_right(&self, delta: f32) {
        let mut pos = self.cursor_pos();
        pos[0] += delta;
        self.set_cursor_pos(pos);
    }
    
    fn set_cursor_x(&self, x: f32) {
        let mut pos = self.cursor_pos();
        pos[0] = x;
        self.set_cursor_pos(pos);
    }
    
    fn set_cursor_y(&self, y: f32) {
        let mut pos = self.cursor_pos();
        pos[1] = y;
        self.set_cursor_pos(pos);
    }
    
    fn cursor_x(&self) -> f32 {
        self.cursor_pos()[0]
    }
    
    fn cursor_y(&self) -> f32 {
        self.cursor_pos()[1]
    }
}
