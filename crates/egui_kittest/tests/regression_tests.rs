use egui::accesskit::{self, Role};
use egui::{
    Align2, Button, ComboBox, FontId, Image, Label, Modifiers, Popup, Pos2, Rect, Stroke,
    StrokeKind, Vec2, Widget as _, Window,
};
#[cfg(all(feature = "wgpu", feature = "snapshot"))]
use egui_kittest::SnapshotResults;
use egui_kittest::{Harness, kittest::Queryable as _};

#[test]
pub fn focus_should_skip_over_disabled_buttons() {
    let mut harness = Harness::new_ui(|ui| {
        ui.add(Button::new("Button 1"));
        ui.add_enabled(false, Button::new("Button Disabled"));
        ui.add(Button::new("Button 3"));
    });

    harness.key_press(egui::Key::Tab);
    harness.run();

    let button_1 = harness.get_by_label("Button 1");
    assert!(button_1.is_focused());

    harness.key_press(egui::Key::Tab);
    harness.run();

    let button_3 = harness.get_by_label("Button 3");
    assert!(button_3.is_focused());

    harness.key_press(egui::Key::Tab);
    harness.run();

    let button_1 = harness.get_by_label("Button 1");
    assert!(button_1.is_focused());
}

#[test]
pub fn focus_should_skip_over_disabled_drag_values() {
    let mut value_1: u16 = 1;
    let mut value_2: u16 = 2;
    let mut value_3: u16 = 3;

    let mut harness = Harness::new_ui(|ui| {
        ui.add(egui::DragValue::new(&mut value_1));
        ui.add_enabled(false, egui::DragValue::new(&mut value_2));
        ui.add(egui::DragValue::new(&mut value_3));
    });

    harness.key_press(egui::Key::Tab);
    harness.run();

    let drag_value_1 = harness.get_by(|node| node.numeric_value() == Some(1.0));
    assert!(drag_value_1.is_focused());

    harness.key_press(egui::Key::Tab);
    harness.run();

    let drag_value_3 = harness.get_by(|node| node.numeric_value() == Some(3.0));
    assert!(drag_value_3.is_focused());
}

#[test]
fn image_failed() {
    let mut harness = Harness::new_ui(|ui| {
        Image::new("file://invalid/path")
            .alt_text("I have an alt text")
            .max_size(Vec2::new(100.0, 100.0))
            .ui(ui);
    });

    harness.run();
    harness.fit_contents();

    #[cfg(all(feature = "wgpu", feature = "snapshot"))]
    harness.snapshot("image_snapshots");
}

#[test]
fn test_combobox() {
    let items = ["Item 1", "Item 2", "Item 3"];
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 200.0))
        .build_ui_state(
            |ui, selected| {
                ComboBox::new("combobox", "Select Something").show_index(
                    ui,
                    selected,
                    items.len(),
                    |idx| *items.get(idx).expect("Invalid index"),
                );
            },
            0,
        );

    harness.run();

    #[cfg(all(feature = "wgpu", feature = "snapshot"))]
    let mut results = SnapshotResults::new();

    #[cfg(all(feature = "wgpu", feature = "snapshot"))]
    results.add(harness.try_snapshot("combobox_closed"));

    let combobox = harness.get_by_role_and_label(Role::ComboBox, "Select Something");
    combobox.click();

    harness.run();

    #[cfg(all(feature = "wgpu", feature = "snapshot"))]
    results.add(harness.try_snapshot("combobox_opened"));

    let item_2 = harness.get_by_role_and_label(Role::Button, "Item 2");
    item_2.click();

    harness.run();

    assert_eq!(harness.state(), &1);

    // Popup should be closed now
    assert!(harness.query_by_label("Item 2").is_none());
}

/// `https://github.com/emilk/egui/issues/7065`
#[test]
pub fn slider_should_move_with_fixed_decimals() {
    let mut value: f32 = 1.0;

    let mut harness = Harness::new_ui(|ui| {
        // Movement on arrow-key is relative to slider width; make the slider wide so the movement becomes small.
        ui.spacing_mut().slider_width = 2000.0;
        ui.add(egui::Slider::new(&mut value, 0.1..=10.0).fixed_decimals(2));
    });

    harness.key_press(egui::Key::Tab);
    harness.run();

    let actual_slider = harness.get_by_role(accesskit::Role::SpinButton);
    assert_eq!(actual_slider.value(), Some("1.00".to_owned()));

    harness.key_press(egui::Key::ArrowRight);
    harness.run();

    let actual_slider = harness.get_by_role(accesskit::Role::SpinButton);
    assert_eq!(actual_slider.value(), Some("1.01".to_owned()));

    harness.key_press(egui::Key::ArrowRight);
    harness.run();

    let actual_slider = harness.get_by_role(accesskit::Role::SpinButton);
    assert_eq!(actual_slider.value(), Some("1.02".to_owned()));

    harness.key_press(egui::Key::ArrowLeft);
    harness.run();

    let actual_slider = harness.get_by_role(accesskit::Role::SpinButton);
    assert_eq!(actual_slider.value(), Some("1.01".to_owned()));

    harness.key_press(egui::Key::ArrowLeft);
    harness.run();

    let actual_slider = harness.get_by_role(accesskit::Role::SpinButton);
    assert_eq!(actual_slider.value(), Some("1.00".to_owned()));
}

#[test]
pub fn override_text_color_affects_interactive_widgets() {
    use egui::{Color32, RichText};

    let mut harness = Harness::new_ui(|ui| {
        _ = ui.button("normal");
        _ = ui.checkbox(&mut true, "normal");
        _ = ui.radio(true, "normal");
        ui.visuals_mut().widgets.inactive.fg_stroke.color = Color32::RED;
        _ = ui.button("red");
        _ = ui.checkbox(&mut true, "red");
        _ = ui.radio(true, "red");
        // override_text_color takes precedence over `WidgetVisuals`, as it docstring claims
        ui.visuals_mut().override_text_color = Some(Color32::GREEN);
        _ = ui.button("green");
        _ = ui.checkbox(&mut true, "green");
        _ = ui.radio(true, "green");
        // Setting the color explicitly with `RichText` overrides style
        _ = ui.button(RichText::new("blue").color(Color32::BLUE));
        _ = ui.checkbox(&mut true, RichText::new("blue").color(Color32::BLUE));
        _ = ui.radio(true, RichText::new("blue").color(Color32::BLUE));
    });

    #[cfg(all(feature = "wgpu", feature = "snapshot"))]
    let mut results = SnapshotResults::new();

    #[cfg(all(feature = "wgpu", feature = "snapshot"))]
    results.add(harness.try_snapshot("override_text_color_interactive"));
}

/// <https://github.com/rerun-io/rerun/issues/11301>
#[test]
pub fn menus_should_close_even_if_submenu_disappears() {
    const OTHER_BUTTON: &str = "Other button";
    const MENU_BUTTON: &str = "Menu";
    const SUB_MENU_BUTTON: &str = "Always here";
    const TOGGLEABLE_SUB_MENU_BUTTON: &str = "Maybe here";
    const INSIDE_SUB_MENU_BUTTON: &str = "Inside submenu";

    for frame_delay in (0..3).rev() {
        let mut harness = Harness::builder().build_ui_state(
            |ui, state| {
                let _ = ui.button(OTHER_BUTTON).clicked();
                let response = ui.button(MENU_BUTTON);

                Popup::menu(&response).show(|ui| {
                    let _ = ui.button(SUB_MENU_BUTTON);
                    if *state {
                        ui.menu_button(TOGGLEABLE_SUB_MENU_BUTTON, |ui| {
                            let _ = ui.button(INSIDE_SUB_MENU_BUTTON);
                        });
                    }
                });
            },
            true,
        );

        // Open the main menu
        harness.get_by_label(MENU_BUTTON).click();
        harness.run();

        // Open the sub menu
        harness
            .get_by_label_contains(TOGGLEABLE_SUB_MENU_BUTTON)
            .hover();
        harness.run();

        // Have we opened the submenu successfully?
        harness.get_by_label(INSIDE_SUB_MENU_BUTTON).hover();
        harness.run();

        // We click manually, since we want to precisely time that the sub menu disappears when the
        // button is released
        let center = harness.get_by_label(OTHER_BUTTON).rect().center();
        harness.input_mut().events.push(egui::Event::PointerButton {
            pos: center,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::default(),
        });
        harness.step();

        // Yank the sub menu from under the pointer
        *harness.state_mut() = false;

        // See if we handle it with or without a frame delay
        harness.run_steps(frame_delay);

        // Actually close the menu by clicking somewhere outside
        harness.input_mut().events.push(egui::Event::PointerButton {
            pos: center,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::default(),
        });

        harness.run();

        assert!(
            harness.query_by_label_contains(SUB_MENU_BUTTON).is_none(),
            "Menu failed to close. frame_delay = {frame_delay}"
        );
    }
}

fn keyboard_submenu_harness() -> Harness<'static, bool> {
    Harness::builder()
        .with_size(Vec2::new(400.0, 240.0))
        .build_ui_state(
            |ui, checked| {
                egui::Panel::top("menu_bar").show(ui, |ui| {
                    egui::MenuBar::new().ui(ui, |ui| {
                        ui.menu_button("X", |ui| {
                            ui.menu_button("Y", |ui| {
                                ui.checkbox(checked, "Goal");
                            });
                        });
                    });
                });
            },
            false,
        )
}

#[test]
pub fn keyboard_should_open_nested_submenu() {
    let mut harness = keyboard_submenu_harness();

    harness.get_by_label("X").focus();
    harness.run();

    harness.key_press(egui::Key::Enter);
    harness.run();

    harness.get_by_label_contains("Y").focus();
    harness.run();

    harness.key_press(egui::Key::Enter);
    harness.run();

    assert!(
        harness.query_by_label("Goal").is_some(),
        "Expected nested submenu to open via keyboard"
    );
}

#[test]
pub fn keyboard_should_close_nested_submenu_with_second_enter() {
    let mut harness = keyboard_submenu_harness();

    harness.get_by_label("X").focus();
    harness.run();

    harness.key_press(egui::Key::Enter);
    harness.run();

    harness.get_by_label_contains("Y").focus();
    harness.run();

    harness.key_press(egui::Key::Enter);
    harness.run();

    assert!(
        harness.query_by_label("Goal").is_some(),
        "Expected nested submenu to open before close attempt"
    );

    harness.get_by_label_contains("Y").focus();
    harness.run();

    harness.key_press(egui::Key::Enter);
    harness.run();

    assert!(
        harness.query_by_label("Goal").is_none(),
        "Expected nested submenu to close when pressing Enter again"
    );
}

/// Regression test for a bug in `horizontal_wrapped` layouts where text wraps but does not
/// move to the next line, causing overlapping text.
///
/// Sweeps the available width from 200 down to 50 (one frame per width) and asserts that no
/// two `TextRun` accesskit nodes (one per laid-out row) have overlapping bounds, and that
/// all accesskit text runs and painted text shapes stay within the `horizontal_wrapped` rect.
#[test]
pub fn horizontal_wrapped_text_should_not_overlap() {
    struct State {
        width: f32,
        rect: egui::Rect,
    }

    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 400.0))
        .build_ui_state(
            |ui, state: &mut State| {
                ui.set_width(state.width);
                state.rect = egui::Frame::popup(ui.style())
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.set_width(ui.available_width());
                            for i in 0..20 {
                                ui.label(format!("Hello{i}"));
                            }
                        })
                        .response
                        .rect
                    })
                    .inner;
            },
            State {
                width: 200.0,
                rect: Rect::NAN,
            },
        );

    let min_width = 50.0;

    loop {
        let width = harness.state().width - 1.0;
        if width < min_width {
            break;
        }
        harness.state_mut().width = width;
        harness.step();

        let container_rect = harness.state().rect.expand(1.0);

        let runs: Vec<_> = harness
            .query_all_by_role(accesskit::Role::TextRun)
            .map(|node| (node.rect(), node.value().unwrap_or_default()))
            .collect();

        for (rect, text) in &runs {
            assert!(
                container_rect.contains_rect(*rect),
                "TextRun rect at available width = {width} is outside horizontal_wrapped rect: \
                 {text:?} {rect:?} outside {container_rect:?}"
            );
        }

        for clipped in &harness.output().shapes {
            if let egui::epaint::Shape::Text(text_shape) = &clipped.shape {
                let shape_rect = text_shape.visual_bounding_rect();
                assert!(
                    container_rect.contains_rect(shape_rect),
                    "TextShape rect at available width = {width} is outside horizontal_wrapped rect: \
                     {:?} {shape_rect:?} outside {container_rect:?}",
                    text_shape.galley.text()
                );
            }
        }

        for i in 0..runs.len() {
            for j in (i + 1)..runs.len() {
                let (a, ta) = &runs[i];
                let (b, tb) = &runs[j];
                let inter = a.intersect(*b);
                // Allow tiny floating-point slop for rects that just touch.
                let overlaps = inter.width() > 0.5 && inter.height() > 0.5;
                assert!(
                    !overlaps,
                    "TextRun rects overlap at available width = {width}: \
                     {ta:?} {a:?} vs {tb:?} {b:?} \
                     (overlap = {}x{})",
                    inter.width(),
                    inter.height()
                );
            }
        }
    }
}

#[test]
pub fn pointer_click_on_open_submenu_button_should_not_close_it() {
    let mut harness = keyboard_submenu_harness();

    harness.get_by_label("X").click();
    harness.run();

    harness.get_by_label_contains("Y").click();
    harness.run();

    assert!(
        harness.query_by_label("Goal").is_some(),
        "Expected submenu to remain open after pointer click on its button"
    );

    harness.get_by_label_contains("Y").click();
    harness.run();

    assert!(
        harness.query_by_label("Goal").is_some(),
        "Expected submenu to remain open on repeated pointer click"
    );
}

/// This test checks if we correctly handle wrapping content proceeding non-wrapping content
/// during window resize. When the window is resized past non-wrapping content, the wrapping content
/// above should stay at that non wrapping width and not wrap any further.
#[test]
fn window_resize_wraps_to_content_min_width() {
    let wrap_text = "This label should wrap as the window is narrowed. \
    It should not shrink smaller than the bottom labels width though.";
    let non_wrap_text = "This is the bottom non-wrapping label which is wider.";

    let window_title = "resize_wrap_regression";
    let mut harness = Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui(move |ui| {
            Window::new(window_title)
                .default_pos([20.0, 20.0])
                .default_size([400.0, 200.0])
                .show(ui.ctx(), |ui| {
                    ui.add(Label::new(wrap_text).wrap());
                    ui.add(Label::new(non_wrap_text).extend());
                });
        });

    harness.run();

    let window_rect = harness
        .get_by_role_and_label(Role::Window, window_title)
        .rect();

    // Drag the right edge inward, well past the non-wrapping label's natural
    // width, so the non-wrapping label pins the window's minimum width while
    // the wrapping label would (without the fix) keep shrinking.
    let grab = Pos2::new(window_rect.right(), window_rect.center().y);
    let target = Pos2::new(window_rect.left() + 80.0, window_rect.center().y);

    harness.drag_at(grab);
    harness.run();
    harness.hover_at(target);

    harness.run();

    let wrap_width = harness.get_by_label(wrap_text).rect().width();
    let non_wrap_width = harness.get_by_label(non_wrap_text).rect().width();

    // Wrapped text won't perfectly fill the available width — each line ends
    // wherever the next word stops fitting. The tolerance absorbs that
    // word-break slack while still catching the bug, where the wrap label
    // would be substantially narrower than the non-wrapping label.
    assert!(
        non_wrap_width - wrap_width < 40.0,
        "wrapping label width ({wrap_width}) is much narrower than the \
         non-wrapping label width ({non_wrap_width}) after shrinking the \
         window past the non-wrapping label's natural width"
    );
}

/// Ensure that the size passed to window is actually treated as outer size (including
/// margins and borders).
#[test]
fn window_fixed_size_is_outer_size() {
    use egui::{Color32, Frame, Margin, Pos2, Shape};

    let outer_pos = Pos2::new(50.0, 50.0);
    let outer_size = Vec2::new(300.0, 200.0);
    let outer_margin = Margin::same(10);
    let expected_rect = Rect::from_min_size(outer_pos, outer_size);

    let mut harness = Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui(move |ui| {
            let frame = Frame::window(ui.style()).outer_margin(outer_margin);
            Window::new("size_test")
                .frame(frame)
                .fixed_pos(outer_pos)
                .fixed_size(outer_size)
                .show(ui.ctx(), |ui| {
                    // Fill the available space so `Resize` doesn't auto-shrink the window
                    // below the requested fixed size.
                    ui.allocate_space(ui.available_size());
                });

            // Paint a debug rect on top of everything that marks the expected outer
            // window rect. In the snapshot this should line up exactly with the
            // painted window frame.
            let painter = ui.ctx().debug_painter();
            painter.rect_stroke(
                expected_rect,
                0.0,
                Stroke::new(2.0, Color32::RED),
                StrokeKind::Outside,
            );
            painter.text(
                expected_rect.left_top() + Vec2::new(0.0, -4.0),
                Align2::LEFT_BOTTOM,
                "should perfectly match the outer window size/position",
                FontId::default(),
                Color32::RED,
            );

            // Also paint the expected *visible frame* rect (outer rect shrunk by the
            // frame's outer_margin). In the snapshot this should line up exactly with
            // the painted window frame.
            let expected_frame_rect = expected_rect - outer_margin;
            painter.debug_rect(
                expected_frame_rect,
                Color32::GREEN,
                "should perfectly match the painted window frame",
            );
        });

    harness.run();

    #[cfg(all(feature = "wgpu", feature = "snapshot"))]
    harness.snapshot("window_outer_size");

    fn collect_filled_rect_sizes(shape: &Shape, out: &mut Vec<Vec2>) {
        match shape {
            // Skip stroke-only rects (fill == TRANSPARENT), so the debug overlay
            // doesn't trivially satisfy the size check.
            Shape::Rect(r) if r.fill != Color32::TRANSPARENT => out.push(r.rect.size()),
            Shape::Vec(v) => v.iter().for_each(|s| collect_filled_rect_sizes(s, out)),
            _ => {}
        }
    }

    let mut sizes = Vec::new();
    for clipped in &harness.output().shapes {
        collect_filled_rect_sizes(&clipped.shape, &mut sizes);
    }

    // The shape will have the inner size
    let painted_size = outer_size - outer_margin.sum();
    let found = sizes
        .iter()
        .any(|s| (s.x - painted_size.x).abs() < 0.5 && (s.y - painted_size.y).abs() < 0.5);

    assert!(
        found,
        "expected a filled RectShape with size {painted_size:?} (outer size {outer_size:?} \
         minus outer margin {outer_margin:?}) in the paint output, but no painted rect matched. \
         Found filled-rect sizes: {sizes:?}"
    );
}
