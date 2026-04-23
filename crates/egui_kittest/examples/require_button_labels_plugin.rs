//! Example that shows how to create a `egui_kittest` Plugin that ensures each button has a label.

use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Plugin};

/// Plugin that panics if any visible button in the current UI lacks a non-empty label.
pub struct RequireButtonLabels;

impl<S> Plugin<S> for RequireButtonLabels {
    fn after_step(&mut self, harness: &mut Harness<'_, S>) {
        for button in harness.query_all_by_role(egui::accesskit::Role::Button) {
            let node = button.accesskit_node();

            match node.label().as_deref() {
                Some(label) if !label.is_empty() => {}
                _ => panic!(
                    "Button at {:?} has no accessible label. \
                     Every button must be labelled so screen readers and kittest queries \
                     can address it.",
                    node.bounding_box(),
                ),
            }
        }
    }
}

fn main() {
    // Check tests below for usages
}

#[test]
fn test_has_label() {
    let mut harness = Harness::builder()
        .with_plugin(RequireButtonLabels)
        .build_ui(|ui| {
            let _ = ui.button("Test");
        });
    harness.run(); // this is fine
}

#[test]
#[should_panic]
fn test_no_label() {
    let mut harness = Harness::builder()
        .with_plugin(RequireButtonLabels)
        .build_ui(|ui| {
            let _ = ui.button(());
        });
    harness.run(); // BOOM
}
