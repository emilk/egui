use egui::accesskit;

pub fn egui_vec2(vec: accesskit::Vec2) -> egui::Vec2 {
    egui::Vec2::new(vec.x as f32, vec.y as f32)
}

pub fn accesskit_vec2(vec: egui::Vec2) -> accesskit::Vec2 {
    accesskit::Vec2 {
        x: vec.x as f64,
        y: vec.y as f64,
    }
}
