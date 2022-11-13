use egui::animation::{AnimationImpl, Easing, Lerp};
use egui::{Color32, Rect, Sense, Vec2, Widget};

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Animations {
    easing: Easing,
    speed: f32,
    to: f32,
    wait_until_done: bool,
}

impl super::Demo for Animations {
    fn name(&self) -> &'static str {
        "Animations"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        use super::View as _;
        let window = egui::Window::new("Animations")
            .default_width(600.0)
            .default_height(400.0)
            .vscroll(false)
            .open(open);
        window.show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for Animations {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let mut animation = ui.ctx().animation().get::<f32>(ui.id().with("anim"));

        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Easing")
                .selected_text(format!("{:?}", self.easing))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.easing, Easing::Linear, "Linear");
                    ui.selectable_value(&mut self.easing, Easing::EaseIn, "EaseIn");
                    ui.selectable_value(&mut self.easing, Easing::EaseOut, "EaseOut");
                    ui.selectable_value(&mut self.easing, Easing::EaseInOut, "EaseInOut");
                    animation.with_easing(self.easing);
                });
            egui::DragValue::new(&mut self.speed).speed(1.0).ui(ui);
            ui.label("Speed");
            egui::Checkbox::new(&mut self.wait_until_done, "Wait until done").ui(ui);
            ui.separator();
            egui::Slider::new(&mut self.to, 0.0..=1.0)
                .text("Position")
                .ui(ui);
        });
        ui.separator();

        if (!self.wait_until_done || animation.is_finished()) && self.to != *animation.target() {
            animation
                .anchor_source()
                .with_target(self.to)
                .start_with_speed(self.speed);
        }

        let width = ui.min_size().x;

        // Draw stack
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 50.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, 0.0, ui.visuals().faint_bg_color);
        let object = Rect::from_min_size(rect.min, Vec2::splat(rect.height()));

        let move_x = rect.width() - object.width();
        ui.painter().rect_filled(
            object.translate(Vec2::new(move_x * self.to, 0.0)),
            0.0,
            ui.visuals().code_bg_color,
        );

        ui.painter().rect_filled(
            object.translate(Vec2::new(move_x * *animation.source(), 0.0)),
            0.0,
            ui.visuals()
                .code_bg_color
                .linear_multiply(1.0 - animation.get_pos() as f32),
        );

        ui.painter().rect_filled(
            object.translate(Vec2::new(move_x * animation.get_value(), 0.0)),
            0.0,
            ui.visuals().text_color(),
        );
    }
}

impl Default for Animations {
    fn default() -> Self {
        Animations {
            easing: Easing::Linear,
            speed: 1.0,
            to: 1.0,
            wait_until_done: false,
        }
    }
}
