use egui::accesskit::Role;
use egui::{Align, Color32, Image, Label, Layout, Modifiers, PointerButton, RichText, Sense, TextWrapMode, include_image};
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

/// Double-clicking a table column resize handle should auto-size the column.
///
/// This was broken because the double-click detection used `ui.id()` to construct
/// the resize handle widget ID, but the actual resize handle widget was created
/// with `state_id` (= `ui.id().with(id_salt)`). The IDs didn't match, so
/// `read_response()` never saw the double-click.
#[test]
fn table_column_resize_double_click_auto_sizes() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 200.0))
        .with_step_dt(0.05)
        .build_ui(|ui| {
            egui_extras::TableBuilder::new(ui)
                .id_salt("resize_dblclick_test")
                .resizable(true)
                .column(egui_extras::Column::initial(200.0).resizable(true))
                .column(egui_extras::Column::remainder())
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.label("Wide Col");
                    });
                    header.col(|ui| {
                        ui.label("Col B");
                    });
                })
                .body(|mut body| {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.label("Short");
                        });
                        row.col(|ui| {
                            ui.label("Other");
                        });
                    });
                });
        });

    // Run a few frames so the table is fully laid out.
    harness.run_steps(3);

    // Column B should start far to the right (column A is 200px wide).
    let col_b_x_before = harness.get_by_label("Col B").rect().left();
    assert!(
        col_b_x_before > 150.0,
        "Col B should start far right (got {col_b_x_before})"
    );

    // Find the resize handle: it's in the spacing gap just left of column B.
    let col_a_header = harness.get_by_label("Wide Col");
    let col_b_header = harness.get_by_label("Col B");
    let handle_x = col_b_header.rect().left() - 4.0;
    let handle_y = col_a_header.rect().center().y;
    let handle_pos = egui::pos2(handle_x, handle_y);

    // Simulate double-click: two clicks in separate frames so egui detects it.
    harness.event(egui::Event::PointerMoved(handle_pos));
    harness.step();

    // First click (press + release).
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: handle_pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::default(),
        });
    }
    harness.step();

    // Second click (press + release) — egui detects this as double-click.
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: handle_pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::default(),
        });
    }

    // Let the auto-size take effect.
    harness.run_steps(5);

    // Column B should have moved left because column A shrunk to fit "Wide Col".
    let col_b_x_after = harness.get_by_label("Col B").rect().left();
    assert!(
        col_b_x_after < col_b_x_before,
        "Col B should have moved left after double-click auto-fit on col A \
         (before: {col_b_x_before}, after: {col_b_x_after})"
    );
}
