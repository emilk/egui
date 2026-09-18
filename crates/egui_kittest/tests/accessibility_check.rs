//! The harness refuses input widgets that have no accessible name.

use egui::{Button, DragValue, Slider, TextEdit};
use egui_kittest::Harness;

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
