use egui::accesskit::Role;
use egui::{Align, Color32, Image, Label, Layout, RichText, Sense, TextWrapMode, include_image};
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
                RichText::new("Long text that should be elided and has lots of styling and is long enough to have multiple lines.")
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
    let mut text = "hello ".to_owned();
    let mut harness = Harness::builder().with_size((200.0, 50.0)).build_ui(|ui| {
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
        harness.snapshot(format!("text_edit_rtl_{i}"));
    }
}

#[test]
fn combobox_should_have_value() {
    let harness = Harness::new_ui(|ui| {
        egui::ComboBox::from_label("Select an option")
            .selected_text("Option 1")
            .show_ui(ui, |_ui| {});
    });

    assert_eq!(
        harness.get_by_label("Select an option").value().as_deref(),
        Some("Option 1")
    );
}

#[test]
fn truncate_for_scaling() {
    let mut harness = Harness::new_ui(|ui| {
        ui.add_sized(
            vec2(60.0, 0.0),
            Button::new("WWWWWWWWWWWWWWWWWWWW").truncate(),
        );
        ui.add_sized(
            vec2(60.0, 0.0),
            Button::new("AAAAAAAAAAAAAAAAAAAA").truncate(),
        );
        ui.add_sized(
            vec2(60.0, 0.0),
            Button::new("nnnnnnnnnnnnnnnnnnnn").truncate(),
        );
        ui.add_sized(
            vec2(60.0, 0.0),
            Button::new("tttttttttttttttttttt").truncate(),
        );
        ui.add_sized(
            vec2(60.0, 0.0),
            Button::new("iiiiiiiiiiiiiiiiiiii").truncate(),
        );
    });

    for scale in [0.5, 1.0, 2.0] {
        harness.set_pixels_per_point(scale);
        harness.step();
        harness.set_size(vec2(76.0, 120.0));
        harness.step();
        harness.snapshot(format!("truncate_for_scaling_{scale}"));
    }
}

/// This test ensures that `ui.response().interact(...)` works correctly.
///
/// This was broken, because there was an optimization in [`egui::Response::interact`]
/// which caused the [`Sense`] of the original response to flip-flop between `click` and `hover`
/// between frames.
///
/// See <https://github.com/emilk/egui/pull/7713> for more details.
#[test]
fn interact_on_ui_response_should_be_stable() {
    let mut first_frame = true;
    let mut click_count = 0;
    let mut harness = Harness::new_ui(|ui| {
        let ui_response = ui.response();
        if !first_frame {
            assert!(
                ui_response.sense.contains(Sense::click()),
                "ui.response() didn't have click sense even though we called interact(Sense::click()) last frame"
            );
        }

        // Add a label so we have something to click with kittest
        ui.add(
            Label::new("senseless label")
                .sense(Sense::hover())
                .selectable(false),
        );

        let click_response = ui_response.interact(Sense::click());
        if click_response.clicked() {
            click_count += 1;
        }
        first_frame = false;
    });

    for i in 0..=10 {
        harness.run_steps(i);
        harness.get_by_label("senseless label").click();
    }

    drop(harness);
    assert_eq!(click_count, 10, "We missed some clicks!");
}
