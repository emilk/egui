use egui::{CollapsingHeader, Pos2, Ui};
use egui_kittest::Harness;
use kittest::Queryable as _;

/// Three headers at the top level, and two more inside "Group".
fn headers(ui: &mut Ui) {
    for name in ["A", "B", "C"] {
        CollapsingHeader::new(name).show(ui, |ui| {
            ui.label(format!("{name} body"));
        });
    }
    CollapsingHeader::new("Group")
        .default_open(true)
        .show(ui, |ui| {
            for name in ["X", "Y"] {
                CollapsingHeader::new(name).show(ui, |ui| {
                    ui.label(format!("{name} body"));
                });
            }
        });
    CollapsingHeader::new("Z").show(ui, |ui| {
        ui.label("Z body");
    });
}

fn headers_in_window(ui: &mut Ui) {
    egui::Window::new("Window").show(ui.ctx(), |ui| {
        egui::ScrollArea::vertical().show(ui, headers);
    });
}

fn center(harness: &Harness<'_>, label: &str) -> Pos2 {
    harness.get_by_label(label).rect().center()
}

fn is_open(harness: &Harness<'_>, name: &str) -> bool {
    harness.query_by_label(&format!("{name} body")).is_some()
}

/// Presses on `from` and drags to each of `over` in turn, a few points at a time like a hand on
/// a mouse, since opening a header pushes the ones below it further down.
fn drag_over(harness: &mut Harness<'_>, from: &str, over: &[&str]) {
    let mut pos = center(harness, from);
    harness.drag_at(pos);
    // While the button is down on a window, it repaints every frame, so `run` never settles.
    harness.run_steps(2);
    for label in over {
        for _ in 0..200 {
            let target = harness.get_by_label(label).rect();
            if target.y_range().contains(pos.y) {
                break;
            }
            pos.y += 4.0 * (target.center().y - pos.y).signum();
            harness.hover_at(pos);
            harness.run_steps(2);
        }
    }
    harness.drop_at(pos);
    harness.run();
}

#[test]
fn dragging_over_headers_opens_them_all() {
    let mut harness = Harness::new_ui(headers);

    drag_over(&mut harness, "A", &["C"]);

    assert!(is_open(&harness, "A"));
    assert!(is_open(&harness, "B"));
    assert!(is_open(&harness, "C"));
}

#[test]
fn dragging_from_an_open_header_closes_the_others() {
    let mut harness = Harness::new_ui(headers);
    for name in ["A", "B"] {
        harness.get_by_label(name).click();
        harness.run();
    }

    drag_over(&mut harness, "B", &["A"]);

    assert!(!is_open(&harness, "A"));
    assert!(!is_open(&harness, "B"));
}

#[test]
fn dragging_leaves_headers_at_other_levels_alone() {
    let mut harness = Harness::new_ui(headers);

    // From X up over its parent "Group", then down past Y to "Z" one level up:
    drag_over(&mut harness, "X", &["Group", "Z"]);

    assert!(is_open(&harness, "X"));
    assert!(is_open(&harness, "Y"));
    assert!(
        harness.query_by_label("X").is_some(),
        "the parent \"Group\" stays open"
    );
    assert!(!is_open(&harness, "Z"));
    assert!(!is_open(&harness, "C"));
}

#[test]
fn dragging_works_inside_a_window() {
    let mut harness = Harness::new_ui(headers_in_window);
    let window_before = harness.get_by_label("Window").rect();

    drag_over(&mut harness, "A", &["C"]);

    assert!(is_open(&harness, "A"));
    assert!(is_open(&harness, "B"));
    assert!(is_open(&harness, "C"));
    assert_eq!(
        harness.get_by_label("Window").rect().min,
        window_before.min,
        "dragging from a header does not move the window"
    );
}

#[test]
fn clicking_still_toggles_only_one_header() {
    let mut harness = Harness::new_ui(headers);

    harness.get_by_label("A").click();
    harness.run();

    assert!(is_open(&harness, "A"));
    assert!(!is_open(&harness, "B"));

    harness.get_by_label("A").click();
    harness.run();

    assert!(!is_open(&harness, "A"));
}
