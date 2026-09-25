use std::collections::BTreeMap;

use egui::{Button, ComboBox, DragValue, TextEdit, accesskit::Role};
use egui_kittest::{Harness, kittest::Queryable as _};

#[derive(Debug)]
struct Heights {
    inner: f32,
    outer: f32,
}

fn measure_height(ui: &mut egui::Ui, f: impl FnOnce(&mut egui::Ui) -> f32) -> Heights {
    ui.style_mut().spacing.interact_size.y = 0.0;
    let mut inner = 0.0;
    let outer = ui
        .horizontal(|ui| {
            inner = f(ui);
        })
        .response
        .rect
        .height();
    Heights { inner, outer }
}

/// With the default style, widgets that commonly sit next to each other in a row
/// should all be the same height, so they line up nicely.
#[test]
fn default_widget_heights_should_match() {
    let mut harness = Harness::builder()
        .with_accessibility_check(false)
        .build_ui_state(
            |ui, heights: &mut BTreeMap<&'static str, Heights>| {
                ui.vertical(|ui| {
                    ui.style_mut().spacing.interact_size.y = 0.0;

                    heights.insert(
                        "drag_value",
                        measure_height(ui, |ui| {
                            let mut value = 42.0;
                            ui.add(DragValue::new(&mut value)).rect.height()
                        }),
                    );

                    heights.insert(
                        "button",
                        measure_height(ui, |ui| ui.add(Button::new("Button")).rect.height()),
                    );

                    heights.insert(
                        "text_edit",
                        measure_height(ui, |ui| {
                            let mut text = String::from("Text");
                            ui.add(TextEdit::singleline(&mut text)).rect.height()
                        }),
                    );

                    heights.insert(
                        "combo_box",
                        measure_height(ui, |ui| {
                            ComboBox::from_id_salt("combo")
                                .selected_text("Combo")
                                .show_ui(ui, |_ui| {})
                                .response
                                .rect
                                .height()
                        }),
                    );
                });
            },
            BTreeMap::new(),
        );
    harness.run();

    let heights = harness.state();
    assert_eq!(heights.len(), 4);
    let interact_height = egui::Style::default().spacing.interact_size.y;

    let (first_name, first) = heights.first_key_value().unwrap();
    for (name, h) in heights {
        assert_eq!(
            h.inner, first.inner,
            "{name} and {first_name} have different heights: {heights:#?}"
        );
        assert_eq!(
            h.inner, h.outer,
            "{name} does not fill the row it is in: {h:?}"
        );
        assert!(
            h.inner <= interact_height,
            "{name} is taller ({}) than the default interact_size.y ({interact_height}), \
             so it will overflow its row and look misaligned",
            h.inner
        );
    }
}

/// A [`ComboBox`] and a [`DragValue`] placed next to each other should occupy exactly the
/// same vertical range, regardless of [`egui::style::Spacing::interact_size`].
#[test]
fn combo_box_and_drag_value_should_line_up_for_any_interact_size() {
    for interact_height in [0.0, 10.0, 18.0, 21.0, 22.0, 30.0] {
        #[derive(Clone, Copy)]
        struct Rects {
            drag_value: egui::Rect,
            combo_box: egui::Rect,
            labelled_combo_box: egui::Rect,
        }

        let mut harness = Harness::builder()
            .with_accessibility_check(false)
            .build_ui_state(
                |ui, rects: &mut Rects| {
                    ui.spacing_mut().interact_size.y = interact_height;
                    ui.horizontal(|ui| {
                        ui.label("Label");
                        let mut value = 42.0;
                        rects.drag_value = ui.add(DragValue::new(&mut value)).rect;
                        rects.combo_box = ComboBox::from_id_salt("combo")
                            .selected_text("Combo")
                            .show_ui(ui, |_ui| {})
                            .response
                            .rect;
                        rects.labelled_combo_box = ComboBox::new("labelled_combo", "Label")
                            .selected_text("Combo")
                            .show_ui(ui, |_ui| {})
                            .response
                            .rect;
                    });
                },
                Rects {
                    drag_value: egui::Rect::NOTHING,
                    combo_box: egui::Rect::NOTHING,
                    labelled_combo_box: egui::Rect::NOTHING,
                },
            );
        harness.run();

        let Rects {
            drag_value,
            combo_box,
            labelled_combo_box,
        } = *harness.state();
        assert_eq!(
            drag_value.y_range(),
            combo_box.y_range(),
            "interact_size.y = {interact_height}: DragValue {drag_value:?} vs ComboBox {combo_box:?}"
        );
        // The response of a labelled combo box also covers the label:
        assert_eq!(
            drag_value.y_range(),
            labelled_combo_box.y_range(),
            "interact_size.y = {interact_height}: DragValue {drag_value:?} vs labelled ComboBox {labelled_combo_box:?}"
        );
    }
}

/// Hovering an open [`ComboBox`] must not change its size.
#[test]
fn open_combo_box_should_not_change_size_when_hovered() {
    let mut harness = Harness::builder()
        .with_accessibility_check(false)
        .build_ui_state(
            |ui, rect: &mut egui::Rect| {
                *rect = ComboBox::from_id_salt("combo")
                    .selected_text("Combo")
                    .show_ui(ui, |ui| {
                        ui.label("Item");
                    })
                    .response
                    .rect;
            },
            egui::Rect::NOTHING,
        );
    harness.run();
    let closed = *harness.state();

    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    let open_hovered = *harness.state();

    // Move the pointer away, but keep the popup open:
    harness.event(egui::Event::PointerMoved(egui::pos2(400.0, 400.0)));
    harness.run();
    let open_not_hovered = *harness.state();

    assert_eq!(closed, open_hovered, "size changed when opened");
    assert_eq!(
        open_hovered, open_not_hovered,
        "size of the open combo box changed with hover"
    );
}

/// The stroke of a [`egui::TextEdit`] changes on hover and focus,
/// but that must not change its size.
#[test]
fn text_edit_size_is_stable_on_hover_and_focus() {
    for margin in [None, Some(egui::Margin::symmetric(8, 2))] {
        let mut text = String::from("Hello");
        let mut harness = Harness::builder()
            .with_accessibility_check(false)
            .build_ui(|ui| {
                let mut text_edit = egui::TextEdit::singleline(&mut text);
                if let Some(margin) = margin {
                    text_edit = text_edit.margin(margin);
                }
                ui.add(text_edit);
            });
        harness.run();
        let idle_rect = harness.get_by_role(Role::TextInput).rect();

        harness.get_by_role(Role::TextInput).hover();
        harness.run();
        let hovered_rect = harness.get_by_role(Role::TextInput).rect();
        assert_eq!(
            idle_rect, hovered_rect,
            "Hovering changed size (margin: {margin:?})"
        );

        harness.get_by_role(Role::TextInput).click();
        harness.run();
        assert!(harness.get_by_role(Role::TextInput).is_focused());
        let focused_rect = harness.get_by_role(Role::TextInput).rect();
        assert_eq!(
            idle_rect, focused_rect,
            "Focusing changed size (margin: {margin:?})"
        );
    }
}

/// A singleline [`TextEdit`] with `clip_text(false)` should expand to make all text visible.
#[test]
fn unclipped_text_edit_should_grow_to_fit_text() {
    let mut harness = Harness::builder()
        .with_accessibility_check(false)
        .build_ui_state(
            |ui, (text_edit_width, text_width): &mut (f32, f32)| {
                let mut text = String::from("This text is much wider than the TextEdit");
                let output = TextEdit::singleline(&mut text)
                    .clip_text(false)
                    .desired_width(0.0)
                    .show(ui);
                *text_edit_width = output.response.rect.width();
                *text_width = output.galley.size().x;
            },
            (0.0, 0.0),
        );
    harness.run();
    let (text_edit_width, text_width) = *harness.state();
    assert!(
        text_width < text_edit_width,
        "TextEdit ({text_edit_width}) is narrower than its text ({text_width})"
    );
}
