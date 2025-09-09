use egui::accesskit::Role;
use egui::{Align, Color32, Image, Label, Layout, RichText, TextWrapMode, include_image};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

#[test]
fn image_button_should_have_alt_text() {
    let harness = Harness::new_ui(|ui| {
        _ = ui.button(
            Image::new(include_image!("../../../crates/eframe/data/icon.png")).alt_text("Egui"),
        );
    });

    harness.get_by_label("Egui");
}

#[test]
fn hovering_should_preserve_text_format() {
    let mut harness = Harness::builder().with_size((200.0, 70.0)).build_ui(|ui| {
        ui.add(
            Label::new(
                RichText::new("Long text that should be elided and has lots of styling")
                    .italics()
                    .underline()
                    .color(Color32::LIGHT_BLUE),
            )
            .wrap_mode(TextWrapMode::Truncate),
        );
    });

    harness.get_by_label_contains("Long text").hover();

    harness.run_steps(5);

    harness.snapshot("hovering_should_preserve_text_format");
}

#[test]
fn text_edit_rtl() {
    let mut text = "hello ".to_string();
    let mut harness = Harness::builder().with_size((300.0, 100.0)).build_ui(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            _ = ui.button("right");
            ui.add(
                egui::TextEdit::singleline(&mut text)
                    .desired_width(10.0)
                    .clip_text(false),
            );
            _ = ui.button("left");
        });
    });

    harness.get_by_role(Role::TextInput).focus();
    harness.step();
    harness.snapshot("text_edit_rtl_0");

    harness.get_by_role(Role::TextInput).type_text("world");

    for i in 1..3 {
        harness.step();
        harness.snapshot(format!("text_edit_rtl_{}", i));
    }
}
