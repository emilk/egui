//! Generates the demo video used by the documentation.

use egui::accesskit::Role;
use egui::{Pos2, Rect, Vec2};
use egui_demo_app::WrapApp;
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessRecordingExt as _, RecordingOptions};

const FRAME_RATE: f32 = 30.0;

fn frames(seconds: f32) -> usize {
    (seconds * FRAME_RATE).round() as usize
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
    let toggle = center(harness, Role::Button, label);
    harness.natural_hover_at(toggle);
    harness
        .get_by_role_and_label(Role::Button, label)
        .click_accesskit();
    harness.step();
}

fn natural_wait_until_gone(harness: &mut Harness<'_, WrapApp>, label: &str, timeout_frames: usize) {
    for _ in 0..timeout_frames {
        if harness.query_by_label(label).is_none() {
            return;
        }
        harness.natural_pause(1);
    }

    panic!("timed out waiting for {label:?} to disappear");
}

fn natural_drag_item(harness: &mut Harness<'_, WrapApp>, window: &str, from: &str, to: &str) {
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
    harness
        .natural_drag_at(from)
        .natural_pause(frames(0.2))
        .natural_drop_at(to)
        .natural_pause(frames(0.3));
}

fn natural_draw_stroke(harness: &mut Harness<'_, WrapApp>, points: &[Pos2]) {
    let Some((&start, rest)) = points.split_first() else {
        return;
    };

    harness.natural_hover_at(start);
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
    harness.natural_pause(frames(0.8));

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
    harness
        .natural_click_at(text_edit)
        .natural_type_text("Hello egui!")
        .natural_pause(frames(0.2));

    let button = first_window_center(harness, GALLERY, Role::Button, "Click me!");
    harness.natural_click_at(button);

    let checkbox = window_center(harness, GALLERY, Role::CheckBox, "Checkbox");
    harness.natural_click_at(checkbox);

    let radio = window_center(harness, GALLERY, Role::RadioButton, "Second");
    harness.natural_click_at(radio);

    let slider = harness
        .get_by_role_and_label(Role::Window, GALLERY)
        .get_by_role(Role::Slider)
        .rect();
    let slider_target = Pos2::new(slider.right() - 8.0, slider.center().y);
    harness
        .natural_drag_at(slider.center())
        .natural_drop_at(slider_target)
        .natural_pause(frames(0.3));

    let collapsing = window_center(
        harness,
        GALLERY,
        Role::Button,
        "Click to see what is hidden!",
    );
    harness
        .natural_click_at(collapsing)
        .natural_pause(frames(0.5));

    // Switch to the drag-and-drop demo and move an item between columns.
    toggle_demo(harness, GALLERY);

    const DRAG_AND_DROP: &str = "✋ Drag and Drop";
    toggle_demo(harness, DRAG_AND_DROP);
    harness.natural_pause(frames(0.3));

    natural_drag_item(harness, DRAG_AND_DROP, "Item A", "Item F");
    natural_drag_item(harness, DRAG_AND_DROP, "Item H", "Item C");
    natural_drag_item(harness, DRAG_AND_DROP, "Item E", "Item J");

    // Draw a smiley in the painting demo.
    toggle_demo(harness, DRAG_AND_DROP);

    const PAINTING: &str = "🖊 Painting";
    toggle_demo(harness, PAINTING);
    harness.natural_pause(frames(0.3));

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
        natural_draw_stroke(harness, stroke);
    }
    harness.natural_pause(frames(0.6));

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
    harness
        .natural_click_at(popup_button)
        .natural_pause(frames(0.3));

    let nested_menu = center(harness, Role::Button, "Popups can have submenus ⏵");
    harness
        .natural_hover_at(nested_menu)
        .natural_pause(frames(0.3));
    let submenu = harness
        .get_all_by_role_and_label(Role::Button, "SubMenu ⏵")
        .next()
        .expect("the nested menu should be visible")
        .rect()
        .center();
    harness.natural_hover_at(submenu).natural_pause(frames(0.5));
    let menu_item = harness
        .get_all_by_role_and_label(Role::Button, "Item")
        .last()
        .expect("the nested menu item should be visible")
        .rect()
        .center();
    harness
        .natural_click_at(menu_item)
        .natural_pause(frames(0.3));
    assert!(
        harness.query_by_label("SubMenu ⏵").is_none(),
        "clicking the nested item should close the menu"
    );

    // End with stacked modal dialogs.
    toggle_demo(harness, POPUPS);

    const MODALS: &str = "🗖 Modals";
    toggle_demo(harness, MODALS);
    let open_user_modal = window_center(harness, MODALS, Role::Button, "Open User Modal");
    harness
        .natural_click_at(open_user_modal)
        .natural_pause(frames(0.3));

    let save = center(harness, Role::Button, "Save");
    harness.natural_click_at(save).natural_pause(frames(0.3));
    let confirm = center(harness, Role::Button, "Yes Please");
    harness.natural_click_at(confirm);
    assert!(
        harness.query_by_label("Saving…").is_some(),
        "confirming should start the save progress"
    );
    natural_wait_until_gone(harness, "Saving…", frames(15.0));
    harness.natural_pause(frames(0.5));
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
/// cargo test -p egui_demo_app --test record_demo_app -- --ignored --nocapture
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
