//! Generates the demo video used by the documentation.

use egui::accesskit::Role;
use egui::{Pos2, Rect, Vec2};
use egui_demo_app::WrapApp;
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, RecordingOptions};

const FRAME_RATE: f32 = 30.0;

fn frames(seconds: f32) -> usize {
    (seconds * FRAME_RATE).round() as usize
}

fn pause(harness: &mut Harness<'_, WrapApp>, seconds: f32) {
    harness.run_steps(frames(seconds));
}

fn hover_at(harness: &mut Harness<'_, WrapApp>, pos: Pos2) {
    harness.hover_at(pos);
    harness.step();
}

fn click_at(harness: &mut Harness<'_, WrapApp>, pos: Pos2) {
    harness.hover_at(pos);
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    harness.step();
}

fn center(harness: &Harness<'_, WrapApp>, role: Role, label: &str) -> Pos2 {
    harness.get_by_role_and_label(role, label).rect().center()
}

fn window_center(harness: &Harness<'_, WrapApp>, window: &str, role: Role, label: &str) -> Pos2 {
    harness
        .get_by_role_and_label(Role::Window, window)
        .get_by_role_and_label(role, label)
        .rect()
        .center()
}

fn first_window_center(
    harness: &Harness<'_, WrapApp>,
    window: &str,
    role: Role,
    label: &str,
) -> Pos2 {
    harness
        .get_by_role_and_label(Role::Window, window)
        .get_all_by_role_and_label(role, label)
        .next()
        .unwrap_or_else(|| panic!("could not find {role:?} {label:?} in {window:?}"))
        .rect()
        .center()
}

fn toggle_demo(harness: &mut Harness<'_, WrapApp>, label: &str) {
    harness
        .get_by_role_and_label(Role::Button, label)
        .scroll_to_me();
    harness.step();
    harness
        .get_by_role_and_label(Role::Button, label)
        .click_accesskit();
    harness.step();
}

fn wait_until_gone(harness: &mut Harness<'_, WrapApp>, label: &str, timeout_frames: usize) {
    for _ in 0..timeout_frames {
        if harness.query_by_label(label).is_none() {
            return;
        }
        harness.step();
    }

    panic!("timed out waiting for {label:?} to disappear");
}

fn drag_item(harness: &mut Harness<'_, WrapApp>, window: &str, from: &str, to: &str) {
    let from = harness
        .get_by_role_and_label(Role::Window, window)
        .get_by_label(from)
        .rect()
        .center();
    let to = harness
        .get_by_role_and_label(Role::Window, window)
        .get_by_label(to)
        .rect()
        .center();
    harness.hover_at(from);
    harness.drag_at(from);
    harness.step();
    pause(harness, 0.2);
    harness.hover_at(to);
    harness.drop_at(to);
    harness.step();
    pause(harness, 0.3);
}

fn draw_stroke(harness: &mut Harness<'_, WrapApp>, points: &[Pos2]) {
    let Some((&start, rest)) = points.split_first() else {
        return;
    };

    harness.hover_at(start);
    harness.drag_at(start);
    harness.step();
    for &point in rest {
        harness.hover_at(point);
        harness.step();
    }
    harness.drop_at(*points.last().expect("a stroke has at least one point"));
    harness.step();
}

fn ellipse_points(center: Pos2, radii: Vec2, start: f32, end: f32, points: usize) -> Vec<Pos2> {
    (0..points)
        .map(|index| {
            let t = index as f32 / (points - 1) as f32;
            let angle = egui::lerp(start..=end, t);
            center + Vec2::angled(angle) * radii
        })
        .collect()
}

fn showcase_demo_app(harness: &mut Harness<'_, WrapApp>) {
    pause(harness, 0.8);

    // Leave only the widget gallery open, then demonstrate several kinds of interaction.
    for window in ["About egui", "🖮 Code Example"] {
        toggle_demo(harness, window);
    }

    const GALLERY: &str = "🗄 Widget Gallery";
    let text_edit = harness
        .get_by_role_and_label(Role::Window, GALLERY)
        .get_by_role(Role::TextInput)
        .rect()
        .center();
    click_at(harness, text_edit);
    harness
        .get_by_role_and_label(Role::Window, GALLERY)
        .get_by_role(Role::TextInput)
        .type_text("Hello egui!");
    harness.step();
    pause(harness, 0.2);

    let button = first_window_center(harness, GALLERY, Role::Button, "Click me!");
    click_at(harness, button);

    let checkbox = window_center(harness, GALLERY, Role::CheckBox, "Checkbox");
    click_at(harness, checkbox);

    let radio = window_center(harness, GALLERY, Role::RadioButton, "Second");
    click_at(harness, radio);

    let slider = harness
        .get_by_role_and_label(Role::Window, GALLERY)
        .get_by_role(Role::Slider)
        .rect();
    let slider_target = Pos2::new(slider.right() - 8.0, slider.center().y);
    harness.hover_at(slider.center());
    harness.drag_at(slider.center());
    harness.hover_at(slider_target);
    harness.drop_at(slider_target);
    harness.step();
    pause(harness, 0.3);

    let collapsing = window_center(
        harness,
        GALLERY,
        Role::Button,
        "Click to see what is hidden!",
    );
    click_at(harness, collapsing);
    pause(harness, 0.5);

    // Switch to the drag-and-drop demo and move an item between columns.
    toggle_demo(harness, GALLERY);

    const DRAG_AND_DROP: &str = "✋ Drag and Drop";
    toggle_demo(harness, DRAG_AND_DROP);
    pause(harness, 0.3);

    drag_item(harness, DRAG_AND_DROP, "Item A", "Item F");
    drag_item(harness, DRAG_AND_DROP, "Item H", "Item C");
    drag_item(harness, DRAG_AND_DROP, "Item E", "Item J");

    // Draw a smiley in the painting demo.
    toggle_demo(harness, DRAG_AND_DROP);

    const PAINTING: &str = "🖊 Painting";
    toggle_demo(harness, PAINTING);
    pause(harness, 0.3);

    let window = harness.get_by_role_and_label(Role::Window, PAINTING).rect();
    let instructions = harness
        .get_by_role_and_label(Role::Window, PAINTING)
        .get_by_label("Paint with your mouse/touch!")
        .rect();
    let canvas = Rect::from_min_max(
        Pos2::new(window.left() + 16.0, instructions.bottom() + 12.0),
        Pos2::new(window.right() - 16.0, window.bottom() - 16.0),
    );
    let face_center = canvas.center();
    let radius = 0.36 * canvas.width().min(canvas.height());

    let face = ellipse_points(
        face_center,
        Vec2::splat(radius),
        0.0,
        core::f32::consts::TAU,
        72,
    );
    let left_eye = ellipse_points(
        face_center + Vec2::new(-0.35 * radius, -0.25 * radius),
        Vec2::splat(0.09 * radius),
        0.0,
        core::f32::consts::TAU,
        18,
    );
    let right_eye = ellipse_points(
        face_center + Vec2::new(0.35 * radius, -0.25 * radius),
        Vec2::splat(0.09 * radius),
        0.0,
        core::f32::consts::TAU,
        18,
    );
    let smile = ellipse_points(
        face_center + Vec2::new(0.0, -0.05 * radius),
        Vec2::new(0.55 * radius, 0.52 * radius),
        0.15 * core::f32::consts::PI,
        0.85 * core::f32::consts::PI,
        36,
    );
    for stroke in [&face, &left_eye, &right_eye, &smile] {
        draw_stroke(harness, stroke);
    }
    pause(harness, 0.6);

    // Open the popup demo and travel through two levels of menus.
    toggle_demo(harness, PAINTING);

    const POPUPS: &str = "❕ Popups";
    toggle_demo(harness, POPUPS);
    let popup_button = window_center(
        harness,
        POPUPS,
        Role::Button,
        "Click, right-click and hover me!",
    );
    click_at(harness, popup_button);
    pause(harness, 0.3);

    let nested_menu = center(harness, Role::Button, "Popups can have submenus ⏵");
    hover_at(harness, nested_menu);
    pause(harness, 0.3);
    let submenu = harness
        .get_all_by_role_and_label(Role::Button, "SubMenu ⏵")
        .next()
        .expect("the nested menu should be visible")
        .rect()
        .center();
    hover_at(harness, submenu);
    pause(harness, 0.5);
    let menu_item = harness
        .get_all_by_role_and_label(Role::Button, "Item")
        .last()
        .expect("the nested menu item should be visible")
        .rect()
        .center();
    click_at(harness, menu_item);
    pause(harness, 0.3);
    assert!(
        harness.query_by_label("SubMenu ⏵").is_none(),
        "clicking the nested item should close the menu"
    );

    // End with stacked modal dialogs.
    toggle_demo(harness, POPUPS);

    const MODALS: &str = "🗖 Modals";
    toggle_demo(harness, MODALS);
    let open_user_modal = window_center(harness, MODALS, Role::Button, "Open User Modal");
    click_at(harness, open_user_modal);
    pause(harness, 0.3);

    let save = center(harness, Role::Button, "Save");
    click_at(harness, save);
    pause(harness, 0.3);
    let confirm = center(harness, Role::Button, "Yes Please");
    click_at(harness, confirm);
    assert!(
        harness.query_by_label("Saving…").is_some(),
        "confirming should start the save progress"
    );
    wait_until_gone(harness, "Saving…", frames(15.0));
    pause(harness, 0.5);
}

/// Keep the scripted queries and interactions covered without requiring a GPU or writing a video.
#[test]
fn recording_script_queries_stay_valid() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(900.0, 600.0))
        .with_step_dt(1.0 / FRAME_RATE)
        .build_eframe(|cc| WrapApp::new(cc));

    showcase_demo_app(&mut harness);
}

/// Run with:
///
/// ```sh
/// KITTEST_RECORD_NATURAL=1 cargo test -p egui_demo_app --test record_demo_app -- --ignored --nocapture
/// ```
#[test]
#[ignore = "generates ../../media/demo.mp4"]
fn record_demo_app() {
    let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../media/demo.mp4");
    let mut harness = Harness::builder()
        .with_size(Vec2::new(900.0, 600.0))
        .with_step_dt(1.0 / FRAME_RATE)
        .wgpu()
        .build_eframe(|cc| WrapApp::new(cc));
    harness.start_recording(RecordingOptions::mp4(output, FRAME_RATE));

    showcase_demo_app(&mut harness);

    harness.finish_recording().expect("finish demo recording");
}
