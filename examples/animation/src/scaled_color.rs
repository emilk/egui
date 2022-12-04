use eframe::epaint::Color32;

pub trait ScaledColor32 {
    fn scale(&self, scale: f32) -> Color32;
}

impl ScaledColor32 for Color32 {
    fn scale(&self, scale: f32) -> Color32 {
        let r = fast_round(f32::from(self.r()) * scale);
        let g = fast_round(f32::from(self.g()) * scale);
        let b = fast_round(f32::from(self.b()) * scale);
        let a = fast_round(f32::from(self.a()) * scale);
        Color32::from_rgba_premultiplied(r, g, b, a)
    }
}

fn fast_round(r: f32) -> u8 {
    (r + 0.5).floor() as _ // rust does a saturating cast since 1.45
}
