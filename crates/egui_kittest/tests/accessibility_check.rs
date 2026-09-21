//! The harness refuses input widgets that have no accessible name.

use egui::accesskit::Role;
use egui::{Button, DragValue, Slider, TextEdit};
use egui_kittest::Harness;
use kittest::{NodeT as _, Queryable as _};

/// The accessible name of every node with the given role.
fn names_of(harness: &Harness<'_>, role: Role) -> Vec<Option<String>> {
    harness
        .get_all_by_role(role)
        .map(|node| node.accesskit_node().label())
        .collect()
}

#[test]
#[should_panic(expected = "1 widget(s) have no accessible name")]
fn unnamed_button_fails_the_harness() {
    let _harness = Harness::new_ui(|ui| {
        let _ = ui.add(Button::new(""));
    });
}

#[test]
fn check_can_be_turned_off() {
    let harness = Harness::builder()
        .with_accessibility_check(false)
        .build_ui(|ui| {
            let _ = ui.add(Button::new(""));
        });

    assert_eq!(harness.unnamed_widgets().len(), 1);
}

#[test]
fn named_inputs_pass() {
    let mut text = String::new();
    let _harness = Harness::new_ui(|ui| {
        let _ = ui.button("Named");
        let _ = ui.add(Button::new("")).on_hover_text("Named by tooltip");
        let _ = ui.add(TextEdit::singleline(&mut text).hint_text("Named by hint"));
        let label = ui.label("Name:");
        let _ = ui.text_edit_singleline(&mut text).labelled_by(label.id);
        let mut value = 0.0;
        let _ = ui.add(DragValue::new(&mut value).prefix("R "));
        let _ = ui.add(Slider::new(&mut value, 0.0..=1.0).text("Amount"));
    });
}

/// A `Slider` without text is named by the caller, and its number field shares that name.
#[test]
fn slider_labelled_by_names_its_number_field() {
    let mut value = 0.5;
    let harness = Harness::new_ui(|ui| {
        let label = ui.label("Amount");
        ui.add(Slider::new(&mut value, 0.0..=1.0))
            .labelled_by(label.id);
    });

    assert_eq!(
        names_of(&harness, Role::Slider),
        [Some("Amount".to_owned())]
    );
    assert_eq!(
        names_of(&harness, Role::SpinButton),
        [Some("Amount".to_owned())]
    );
}

/// The slider keeps its name while its number field has focus,
/// even though the returned `Response` then belongs to the number field.
#[test]
fn slider_named_by_tooltip_keeps_name_when_number_field_has_focus() {
    let mut value = 0.5;
    let mut harness = Harness::new_ui(|ui| {
        ui.add(Slider::new(&mut value, 0.0..=1.0))
            .on_hover_text("Amount");
    });

    harness.key_press(egui::Key::Tab); // Focus the slider
    harness.run();
    harness.key_press(egui::Key::Tab); // Focus the number field
    harness.run();

    assert!(
        harness
            .get_by_role(Role::SpinButton)
            .accesskit_node()
            .is_focused(),
        "The number field should have focus"
    );
    assert_eq!(
        names_of(&harness, Role::Slider),
        [Some("Amount".to_owned())]
    );
    assert_eq!(
        names_of(&harness, Role::SpinButton),
        [Some("Amount".to_owned())]
    );
}

/// An explicit label wins over the name a `DragValue` gives itself from its suffix.
#[test]
fn labelled_by_wins_over_drag_value_suffix() {
    let mut value = 3.0;
    let harness = Harness::new_ui(|ui| {
        let label = ui.label("Width");
        ui.add(DragValue::new(&mut value).suffix(" px"))
            .labelled_by(label.id);
        ui.add(DragValue::new(&mut value).prefix("x: ").suffix(" px"));
    });

    assert_eq!(
        names_of(&harness, Role::SpinButton),
        [Some("Width".to_owned()), Some("x: px".to_owned())]
    );
}

#[test]
fn accessible_name_names_a_widget_without_a_label() {
    let mut text = String::new();
    let mut harness = Harness::new_ui(|ui| {
        ui.text_edit_singleline(&mut text).accessible_name("Search");
    });
    harness.run();
    harness.get_by_label("Search");
}

/// A row label can name every unnamed input next to it.
#[test]
fn inputs_can_be_labelled_by_a_row_label() {
    let mut value = 0.5;
    let mut harness = Harness::new_ui(|ui| {
        ui.horizontal(|ui| {
            let label = ui.label("Opacity");
            ui.label_inputs_by(label.id, |ui| {
                ui.add(DragValue::new(&mut value));
            });
        });
    });
    harness.run();
    harness.get_by_role_and_label(Role::SpinButton, "Opacity");
}

#[test]
fn inputs_can_be_named() {
    let mut text = "let x = 1;".to_owned();
    let mut harness = Harness::new_ui(|ui| {
        ui.name_inputs("Code block", |ui| {
            ui.add(TextEdit::multiline(&mut text).interactive(false));
        });
    });
    harness.run();
    harness.get_by_role_and_label(Role::MultilineTextInput, "Code block");
}

/// A [`egui::CollapsingHeader`] is clickable and toggleable, so it counts as an input:
/// one without a heading cannot be reached by name.
#[test]
fn unnamed_collapsing_header_is_an_unnamed_input() {
    let harness = Harness::builder()
        .with_accessibility_check(false)
        .build_ui(|ui| {
            egui::CollapsingHeader::new("").show(ui, |ui| {
                let _ = ui.button("Inside");
            });
        });

    assert_eq!(harness.unnamed_widgets().len(), 1);
}

/// …and it can be named like any other input.
#[test]
fn collapsing_header_can_be_named() {
    let mut harness = Harness::new_ui(|ui| {
        ui.name_inputs("Details", |ui| {
            egui::CollapsingHeader::new("").show(ui, |ui| {
                let _ = ui.button("Inside");
            });
        });
    });
    harness.run();

    harness.get_by_role_and_label(Role::DisclosureTriangle, "Details");
}

/// A text field with a hint already has a name (the placeholder), so it is left alone.
#[test]
fn hinted_text_field_keeps_its_placeholder_name() {
    let mut text = String::new();
    let mut harness = Harness::new_ui(|ui| {
        ui.name_inputs("Field", |ui| {
            ui.add(TextEdit::singleline(&mut text).hint_text("Search"));
        });
    });
    harness.run();
    let field = harness.get_by_role(Role::TextInput).accesskit_node();
    assert_eq!(
        field.label(),
        None,
        "The placeholder is the name; nothing to add"
    );
    assert_eq!(field.placeholder(), Some("Search"));
}
