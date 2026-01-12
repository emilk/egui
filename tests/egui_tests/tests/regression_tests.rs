use std::sync::Arc;

use egui::accesskit::Role;
use egui::text::{LayoutJob, TextWrapping};
use egui::{
    Align, Color32, FontFamily, FontId, Image, Label, Layout, RichText, Sense, TextBuffer,
    TextFormat, TextWrapMode, Ui, include_image, vec2,
};
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
fn text_edit_halign() {
    let mut harness = Harness::builder().with_size((212.0, 212.0)).build_ui(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
        fn layouter(halign: Align) -> impl FnMut(&Ui, &dyn TextBuffer, f32) -> Arc<egui::Galley> {
            move |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
                let mut job = LayoutJob {
                    wrap: TextWrapping {
                        max_rows: 4,
                        max_width: wrap_width,
                        ..Default::default()
                    },
                    halign,
                    ..Default::default()
                };
                job.append(
                    buf.as_str(),
                    0.0,
                    TextFormat::simple(FontId::new(13.0, FontFamily::Proportional), Color32::GRAY),
                );
                ui.fonts_mut(|f| f.layout_job(job))
            }
        }

        for widget_alignment in [Align::Min, Align::Center, Align::Max] {
            ui.horizontal(|ui| {
                for text_alignment in [Align::LEFT, Align::Center, Align::RIGHT] {
                    ui.add_sized(
                        vec2(64.0, 64.0),
                        egui::TextEdit::multiline(&mut format!(
                            "{:?}\n+\n{:?}",
                            widget_alignment, text_alignment,
                        ))
                        .layouter(&mut layouter(text_alignment))
                        .vertical_align(widget_alignment)
                        .horizontal_align(widget_alignment),
                    );
                }
            });
        }
    });

    harness.get_by_value("Center\n+\nCenter").focus();
    harness.step();
    harness.snapshot("text_edit_halign");
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
