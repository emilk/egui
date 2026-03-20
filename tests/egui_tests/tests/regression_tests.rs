use egui::accesskit::Role;
use egui::epaint::Shape;
use egui::style::ScrollAnimation;
use egui::{
    Align, Color32, Image, Label, Layout, RichText, ScrollArea, Sense, TextWrapMode, include_image,
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
fn text_edit_delay() {
    let mut text = String::new();
    let mut harness = Harness::builder().with_size((200.0, 50.0)).build_ui(|ui| {
        ui.style_mut().scroll_animation = ScrollAnimation::none();
        ui.add(egui::TextEdit::singleline(&mut text).hint_text("Write something"));
    });

    harness.get_by_role(Role::TextInput).focus();
    harness.step();
    harness.snapshot("text_edit_delay_0_empty");

    harness.get_by_role(Role::TextInput).type_text("h");

    // When the text is empty, and we show the hint text, there is a frame delay.
    harness.step();
    harness.snapshot("text_edit_delay_1_h_invisible");

    // Now it should be visible
    harness.step();
    harness.snapshot("text_edit_delay_2_h_visible");

    harness.get_by_role(Role::TextInput).type_text("i");

    // The "i" should immediately be visible without a delay
    harness.step();
    harness.snapshot("text_edit_delay_3_i_visible");

    // The next frame should exactly match the previous one
    harness.step();
    harness.snapshot("text_edit_delay_4_i_visible");
}

#[test]
fn text_edit_scroll() {
    let mut text = "1\n2\n3\n4\n".to_owned();
    let mut harness = Harness::builder().build_ui(|ui| {
        ScrollArea::vertical().max_height(40.0).show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .desired_rows(2)
                    .hint_text("Write something"),
            );
        });
    });

    harness.fit_contents();

    harness.get_by_role(Role::MultilineTextInput).focus();
    harness.step();
    harness.snapshot("text_edit_scroll_0_focus");

    harness
        .get_by_role(Role::MultilineTextInput)
        .type_text("5\n");

    // When the text is empty, and we show the hint text, there is a frame delay.
    harness.run();
    harness.snapshot("text_edit_scroll_1_5");
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

fn has_red_warning_rect(output: &egui::FullOutput) -> bool {
    output.shapes.iter().any(|clipped| {
        matches!(
            &clipped.shape,
            Shape::Rect(rect_shape)
                if rect_shape.stroke.color == Color32::RED
        )
    })
}

/// A button that changes its text on hover, with the Id derived from the text.
/// This is a plausible bug: the widget keeps the same rect, but its Id changes
/// between frames because the label (and thus the Id salt) changes on hover.
/// The `warn_if_rect_changes_id` debug check should catch this.
#[test]
fn warn_if_rect_changes_id() {
    let button_rect = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(100.0, 30.0));

    let mut harness = Harness::builder().with_size((200.0, 50.0)).build_ui(|ui| {
        // Simulate a buggy widget whose Id depends on its label text,
        // and the label changes on hover:
        let is_hovered = ui.rect_contains_pointer(button_rect);
        let label = if is_hovered { "Hovering!" } else { "Click me" };
        let id = ui.id().with(label);
        let _response = ui.interact(button_rect, id, Sense::click());
    });

    // no hover — establishes stable prev_pass
    harness.step();
    assert!(
        !has_red_warning_rect(harness.output()),
        "Should not warn without hover"
    );

    // Move the pointer over the button
    harness.hover_at(button_rect.center());

    harness.step();
    assert!(
        has_red_warning_rect(harness.output()),
        "Should warn when a widget rect changes Id between passes"
    );
}
