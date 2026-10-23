//! Dragging a text selection past the edge of a [`ScrollArea`] scrolls it,
//! so that the selection can grow beyond what is visible.

use core::fmt::Write as _;
use std::sync::Arc;

use egui::mutex::Mutex;
use egui::scroll_area::State;
use egui::{
    Event, Id, Modifiers, MouseWheelSource, MouseWheelUnit, OutputCommand, PointerButton, Pos2,
    Rect, ScrollArea, TextEdit, TouchPhase, Vec2, pos2,
};
use egui_kittest::Harness;

const SIZE: Vec2 = Vec2::new(200.0, 100.0);
const ROWS: usize = 100;

fn text_edit_id() -> Id {
    Id::unique("drag-select-text-edit")
}

/// The latest [`State`] and viewport ([`egui::scroll_area::ScrollAreaOutput::inner_rect`])
/// of the scroll area, updated after each frame.
type SharedState = Arc<Mutex<Option<(State, Rect)>>>;

fn text() -> String {
    let mut text = String::new();
    for i in 0..ROWS {
        writeln!(text, "Line {i}").expect("writing to a String cannot fail");
    }
    text
}

/// A vertically scrollable area filling the window, with `ROWS` selectable labels,
/// returning the latest [`State`] of the scroll area after each frame.
fn label_harness() -> (Harness<'static, String>, SharedState) {
    let state: SharedState = Default::default();
    let state_clone = Arc::clone(&state);

    let mut harness = Harness::builder()
        .with_size(SIZE)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, _| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let output = ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    for i in 0..ROWS {
                        ui.label(format!("Row {i}"));
                    }
                });
                *state_clone.lock() = Some((output.state, output.inner_rect));
            },
            String::new(),
        );

    harness.run_steps(2); // Let the scroll area measure its content.
    (harness, state)
}

/// Like [`label_harness`], but with one multiline [`TextEdit`] holding `ROWS` lines.
fn text_edit_harness() -> (Harness<'static, String>, SharedState) {
    let state: SharedState = Default::default();
    let state_clone = Arc::clone(&state);

    let mut harness = Harness::builder()
        .with_size(SIZE)
        .with_step_dt(1.0 / 60.0)
        .with_accessibility_check(false) // the text edit is unlabelled
        .build_ui_state(
            move |ui, text| {
                let output = ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    ui.add(TextEdit::multiline(text).id(text_edit_id()));
                });
                *state_clone.lock() = Some((output.state, output.inner_rect));
            },
            text(),
        );

    harness.run_steps(2);
    (harness, state)
}

fn offset(state: &SharedState) -> Vec2 {
    state
        .lock()
        .as_ref()
        .expect("ScrollArea should have been shown")
        .0
        .clamped_offset()
}

/// The visible part of the scroll area.
fn viewport(state: &SharedState) -> Rect {
    state
        .lock()
        .as_ref()
        .expect("ScrollArea should have been shown")
        .1
}

/// Press at `pos`, then move the pointer to `drag_to` and hold it there for `frames` frames.
fn press_and_hold(harness: &mut Harness<'_, String>, pos: Pos2, drag_to: Pos2, frames: usize) {
    harness.hover_at(pos);
    harness.step();
    harness.drag_at(pos);
    harness.step();
    harness.hover_at(drag_to);
    harness.run_steps(frames);
}

fn release(harness: &mut Harness<'_, String>, pos: Pos2) {
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();
}

/// The text reported as a settled selection in the last pass, if any.
fn reported_selection(harness: &Harness<'_, String>) -> Option<String> {
    harness
        .output()
        .platform_output
        .commands
        .iter()
        .find_map(|command| match command {
            OutputCommand::TextSelectionSettled(text) => Some(text.clone()),
            _ => None,
        })
}

/// Send a copy event and return what was copied, if anything.
fn copy(harness: &mut Harness<'_, String>) -> Option<String> {
    harness.event(Event::Copy);
    harness.step();
    harness
        .output()
        .platform_output
        .commands
        .iter()
        .find_map(|command| match command {
            OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
}

#[test]
fn dragging_a_label_selection_below_the_scroll_area_scrolls_it() {
    let (mut harness, state) = label_harness();
    assert_eq!(offset(&state), Vec2::ZERO);

    let start = pos2(20.0, 30.0);
    let below = pos2(20.0, SIZE.y + 20.0);
    press_and_hold(&mut harness, start, below, 10);
    let after_10 = offset(&state).y;
    assert!(after_10 > 0.0, "Should have scrolled down, got {after_10}");

    // Holding still keeps scrolling:
    harness.run_steps(10);
    let after_20 = offset(&state).y;
    assert!(
        after_20 > after_10,
        "Should keep scrolling while the pointer is held past the edge, got {after_10} -> {after_20}"
    );

    // The selection has grown to include rows that were out of view when the drag started:
    release(&mut harness, below);
    let selected = reported_selection(&harness).expect("Should report the selection");
    assert!(
        selected.contains("Row 10"),
        "Should have selected rows that were below the viewport, got {selected:?}"
    );
}

#[test]
fn dragging_a_label_selection_above_the_scroll_area_scrolls_it_back_up() {
    let (mut harness, state) = label_harness();

    // Scroll down first, by dragging below:
    let start = pos2(20.0, 30.0);
    press_and_hold(&mut harness, start, pos2(20.0, SIZE.y + 20.0), 20);
    let scrolled_down = offset(&state).y;
    assert!(scrolled_down > 0.0);

    // Now hold the pointer above the top edge:
    harness.hover_at(pos2(20.0, -20.0));
    harness.run_steps(10);
    let scrolled_up = offset(&state).y;
    assert!(
        scrolled_up < scrolled_down,
        "Should scroll back up, got {scrolled_down} -> {scrolled_up}"
    );
}

#[test]
fn dragging_within_the_scroll_area_does_not_scroll() {
    let (mut harness, state) = label_harness();

    let start = pos2(20.0, 30.0);
    let inside = pos2(20.0, viewport(&state).bottom() - 2.0);
    press_and_hold(&mut harness, start, inside, 20);
    assert_eq!(
        offset(&state),
        Vec2::ZERO,
        "Should not scroll while the pointer is inside the viewport"
    );
}

#[test]
fn a_label_selection_survives_scrolling_its_anchor_out_of_view() {
    let (mut harness, state) = label_harness();

    // Select from the first row into the second
    let start = pos2(20.0, 30.0);
    let end = pos2(20.0, 40.0);
    press_and_hold(&mut harness, start, end, 1);
    release(&mut harness, end);
    let selected = reported_selection(&harness).expect("Should report the selection");
    assert!(
        selected.contains('\n'),
        "Should span two labels, got {selected:?}"
    );

    // Scroll the selection well out of view with the mouse wheel
    harness.hover_at(pos2(100.0, 50.0));
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: Vec2::new(0.0, -5.0 * SIZE.y),
        modifiers: Modifiers::NONE,
        phase: TouchPhase::Move,
        source: MouseWheelSource::Wheel,
    });
    harness.run_steps(30);
    assert!(
        offset(&state).y > 3.0 * SIZE.y,
        "Should have scrolled far down, got {}",
        offset(&state).y
    );

    // The selection is still there, and copies the right text
    let copied = copy(&mut harness).expect("Should still have a selection to copy");
    assert_eq!(copied, selected);
}

#[test]
fn scrolls_faster_the_further_out_the_pointer_is() {
    let start = pos2(20.0, 30.0);
    let frames = 10;

    let (mut harness, state) = label_harness();
    press_and_hold(&mut harness, start, pos2(20.0, SIZE.y + 2.0), frames);
    let near = offset(&state).y;

    let (mut harness, state) = label_harness();
    press_and_hold(&mut harness, start, pos2(20.0, SIZE.y + 60.0), frames);
    let far = offset(&state).y;

    assert!(
        0.0 < near && near < far,
        "Further out should scroll faster, got {near} (2 pt out) vs {far} (60 pt out)"
    );
}

#[test]
fn dragging_a_text_edit_selection_below_the_scroll_area_scrolls_it() {
    let (mut harness, state) = text_edit_harness();
    assert_eq!(offset(&state), Vec2::ZERO);

    let start = pos2(20.0, 20.0);
    let below = pos2(20.0, SIZE.y + 20.0);
    press_and_hold(&mut harness, start, below, 20);
    assert!(
        offset(&state).y > 0.0,
        "Should have scrolled down, got {}",
        offset(&state).y
    );

    // The selection has grown to include lines that were out of view when the drag started
    let edit_state = TextEdit::load_state(&harness.ctx, text_edit_id()).expect("TextEdit state");
    let range = edit_state
        .cursor
        .char_range()
        .expect("Should have a selection");
    let all = text();
    let selected = range.slice_str(&all);
    assert!(
        selected.contains("Line 10"),
        "Should have selected lines that were below the viewport, got {selected:?}"
    );
}

/// A harness around an arbitrary UI, so that layouts with several scroll areas can be built.
fn harness_with(build: impl FnMut(&mut egui::Ui) + 'static) -> Harness<'static, String> {
    let mut build = build;
    let mut harness = Harness::builder()
        .with_size(SIZE)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(move |ui, _| build(ui), String::new());
    harness.run_steps(2); // Let the scroll areas measure their content.
    harness
}

fn store(state: &SharedState, output: &egui::scroll_area::ScrollAreaOutput<()>) {
    *state.lock() = Some((output.state, output.inner_rect));
}

#[test]
fn scrolls_the_scroll_area_being_dragged_in_not_its_neighbour() {
    let left: SharedState = Default::default();
    let right: SharedState = Default::default();
    let (l, r) = (Arc::clone(&left), Arc::clone(&right));
    let mut harness = harness_with(move |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        ui.columns(2, |columns| {
            let output = ScrollArea::vertical()
                .id_salt("left")
                .auto_shrink(false)
                .show(&mut columns[0], |ui| {
                    for i in 0..ROWS {
                        ui.label(format!("Left {i}"));
                    }
                });
            store(&l, &output);
            let output = ScrollArea::vertical()
                .id_salt("right")
                .auto_shrink(false)
                .show(&mut columns[1], |ui| {
                    for i in 0..ROWS {
                        ui.label(format!("Right {i}"));
                    }
                });
            store(&r, &output);
        });
    });

    let start = pos2(viewport(&right).left() + 10.0, 30.0);
    let below = pos2(start.x, SIZE.y + 20.0);
    press_and_hold(&mut harness, start, below, 20);

    assert!(
        offset(&right).y > 0.0,
        "The right scroll area, where the drag is, should scroll, got {}",
        offset(&right).y
    );
    assert_eq!(
        offset(&left).y,
        0.0,
        "The left scroll area, where nothing is selected, should not scroll"
    );
}

#[test]
fn scrolls_the_scroll_area_the_labels_are_in_not_a_nested_one() {
    let outer: SharedState = Default::default();
    let inner: SharedState = Default::default();
    let (o, i) = (Arc::clone(&outer), Arc::clone(&inner));
    let mut harness = harness_with(move |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let output = ScrollArea::vertical()
            .id_salt("outer")
            .auto_shrink(false)
            .show(ui, |ui| {
                for row in 0..4 {
                    ui.label(format!("Top {row}"));
                }
                let output = ScrollArea::vertical()
                    .id_salt("inner")
                    .max_height(40.0)
                    .show(ui, |ui| {
                        for row in 0..ROWS {
                            ui.label(format!("Inner {row}"));
                        }
                    });
                store(&i, &output);
                for row in 0..ROWS {
                    ui.label(format!("Bottom {row}"));
                }
            });
        store(&o, &output);
    });

    // Drag from the top labels (in the outer area) past the bottom edge:
    let start = pos2(20.0, viewport(&outer).top() + 2.0);
    let below = pos2(20.0, SIZE.y + 20.0);
    press_and_hold(&mut harness, start, below, 20);

    assert!(
        offset(&outer).y > 0.0,
        "The outer scroll area, holding the dragged labels, should scroll, got {}",
        offset(&outer).y
    );
    assert_eq!(
        offset(&inner).y,
        0.0,
        "The nested scroll area, which the drag never touched, should not scroll"
    );
}

#[test]
fn labels_outside_any_scroll_area_do_not_scroll_an_unrelated_one() {
    let unrelated: SharedState = Default::default();
    let u = Arc::clone(&unrelated);
    let mut harness = harness_with(move |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        ui.columns(2, |columns| {
            // Plain labels, not in any scroll area:
            for row in 0..3 {
                columns[0].label(format!("Plain {row}"));
            }
            // An unrelated scroll area beside them, later in the same frame:
            let output = ScrollArea::vertical()
                .id_salt("unrelated")
                .auto_shrink(false)
                .show(&mut columns[1], |ui| {
                    for row in 0..ROWS {
                        ui.label(format!("Unrelated {row}"));
                    }
                });
            store(&u, &output);
        });
    });

    // Drag from the plain labels down past the bottom of the window, staying beside the area:
    let start = pos2(20.0, 20.0);
    let below = pos2(20.0, SIZE.y + 20.0);
    press_and_hold(&mut harness, start, below, 20);

    assert_eq!(
        offset(&unrelated),
        Vec2::ZERO,
        "A drag among labels outside any scroll area should not scroll an unrelated one"
    );
    assert!(
        !harness.ctx.has_requested_repaint(),
        "Nothing can scroll, so nothing should keep requesting repaints"
    );
}

#[test]
fn zoomed_scene_does_not_scroll_while_the_pointer_is_inside_the_viewport() {
    let state: SharedState = Default::default();
    let s = Arc::clone(&state);
    // A scene rect half the window size, so the content is shown zoomed in about 2x:
    let scene_rect = Arc::new(Mutex::new(Rect::from_min_size(Pos2::ZERO, SIZE / 2.0)));
    let scene_rect_clone = Arc::clone(&scene_rect);
    let mut harness = harness_with(move |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let mut scene_rect = *scene_rect_clone.lock();
        egui::Scene::new()
            .sense(egui::Sense::hover()) // no panning, so the labels get the drag
            .show(ui, &mut scene_rect, |ui| {
                let output = ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    for row in 0..ROWS {
                        ui.label(format!("Row {row}"));
                    }
                });
                store(&s, &output);
            });
        *scene_rect_clone.lock() = scene_rect;
    });

    // The scroll area fills the (zoomed) scene, so on screen its viewport is the whole
    // central panel. Hold the pointer well inside it, near the bottom:
    let start = pos2(50.0, 30.0);
    let inside = pos2(50.0, SIZE.y - 12.0);
    press_and_hold(&mut harness, start, inside, 20);

    assert_eq!(
        offset(&state),
        Vec2::ZERO,
        "The pointer is inside the viewport on screen, so nothing should scroll"
    );
}

#[test]
fn scrolling_stops_when_the_button_is_released() {
    let (mut harness, state) = label_harness();
    let start = pos2(20.0, 30.0);
    let below = pos2(20.0, SIZE.y + 20.0);
    press_and_hold(&mut harness, start, below, 10);
    assert!(offset(&state).y > 0.0);

    release(&mut harness, below);
    let at_release = offset(&state).y;
    harness.run_steps(3);
    assert_eq!(
        offset(&state).y,
        at_release,
        "Should not keep scrolling after the button is released"
    );
}

#[test]
fn single_widget_selection_does_not_scroll_past_its_label() {
    // With `multi_widget_text_select` off the selection cannot leave the label it started in,
    // so there is nothing to scroll towards:
    let state: SharedState = Default::default();
    let s = Arc::clone(&state);
    let mut harness = harness_with(move |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        ui.style_mut().interaction.multi_widget_text_select = false;
        let output = ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            for row in 0..ROWS {
                ui.label(format!("Row {row}"));
            }
        });
        store(&s, &output);
    });

    press_and_hold(
        &mut harness,
        pos2(20.0, 30.0),
        pos2(20.0, SIZE.y + 20.0),
        20,
    );
    assert_eq!(offset(&state), Vec2::ZERO);
}

#[test]
fn dragging_past_the_right_edge_scrolls_horizontally() {
    let state: SharedState = Default::default();
    let s = Arc::clone(&state);
    let mut harness = harness_with(move |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let output = ScrollArea::horizontal().auto_shrink(false).show(ui, |ui| {
            for row in 0..3 {
                let mut long = String::new();
                for i in 0..40 {
                    write!(long, "word{row}_{i} ").expect("writing to a String cannot fail");
                }
                ui.add(egui::Label::new(long).extend());
            }
        });
        store(&s, &output);
    });

    press_and_hold(
        &mut harness,
        pos2(20.0, 20.0),
        pos2(SIZE.x + 20.0, 20.0),
        20,
    );
    assert!(
        offset(&state).x > 0.0,
        "Should have scrolled right, got {}",
        offset(&state).x
    );
}

#[test]
fn dragging_a_hyperlink_selection_below_the_scroll_area_scrolls_it() {
    let state: SharedState = Default::default();
    let s = Arc::clone(&state);
    let mut harness = harness_with(move |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let output = ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            for row in 0..ROWS {
                ui.hyperlink_to(format!("Link {row}"), "https://example.com/");
            }
        });
        store(&s, &output);
    });

    press_and_hold(
        &mut harness,
        pos2(20.0, 30.0),
        pos2(20.0, SIZE.y + 20.0),
        20,
    );
    assert!(
        offset(&state).y > 0.0,
        "Should have scrolled down, got {}",
        offset(&state).y
    );
}

#[test]
fn dragging_inside_the_viewport_does_not_scroll_with_clipped_cells() {
    // Like table cells with clipping on: each label's own clip rect is just its row,
    // but the pointer is still well inside the scroll area's viewport.
    let state: SharedState = Default::default();
    let s = Arc::clone(&state);
    let mut harness = harness_with(move |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let output = ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            for row in 0..ROWS {
                let rect = ui.available_rect_before_wrap();
                let rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 14.0));
                ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    ui.shrink_clip_rect(rect);
                    ui.label(format!("Cell {row}"));
                });
            }
        });
        store(&s, &output);
    });

    press_and_hold(&mut harness, pos2(20.0, 20.0), pos2(20.0, 60.0), 30);
    assert_eq!(
        offset(&state),
        Vec2::ZERO,
        "The pointer is inside the viewport, so nothing should scroll"
    );
}

#[test]
fn hidden_labels_are_not_part_of_a_selection() {
    use egui_kittest::kittest::Queryable as _;

    let mut harness = harness_with(|ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        ui.label("First");
        ui.add_visible(false, egui::Label::new("Hidden"));
        ui.label("Third");
    });

    let first = harness.get_by_label("First").rect();
    let third = harness.get_by_label("Third").rect();
    let start = pos2(first.left() + 1.0, first.center().y);
    let end = pos2(third.right() - 1.0, third.center().y);
    press_and_hold(&mut harness, start, end, 2);
    release(&mut harness, end);

    let copied = copy(&mut harness).expect("Should have a selection to copy");
    assert!(
        !copied.contains("Hidden"),
        "Hidden text should not be copied, got {copied:?}"
    );
}

#[test]
fn keeps_requesting_repaints_while_the_pointer_is_held_past_the_edge() {
    let (mut harness, state) = label_harness();
    press_and_hold(&mut harness, pos2(20.0, 30.0), pos2(20.0, SIZE.y + 20.0), 5);
    let before = offset(&state).y;
    harness.step();
    assert!(offset(&state).y > before, "Should still be scrolling");
    assert!(
        harness.ctx.has_requested_repaint(),
        "The pointer is held still, so only a repaint request keeps the scrolling going"
    );
}

#[test]
fn a_scene_inside_a_scroll_area_does_not_scroll_while_the_pointer_is_inside() {
    let outer: SharedState = Default::default();
    let o = Arc::clone(&outer);
    // A scene rect twice the window size, so the content is shown zoomed out:
    let scene_rect = Arc::new(Mutex::new(Rect::from_min_size(Pos2::ZERO, SIZE * 2.0)));
    let scene_rect_clone = Arc::clone(&scene_rect);
    let mut harness = harness_with(move |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let output = ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            let mut scene_rect = *scene_rect_clone.lock();
            egui::Scene::new()
                .sense(egui::Sense::hover()) // no panning, so the labels get the drag
                .max_inner_size(Vec2::new(SIZE.x, SIZE.y - 20.0))
                .show(ui, &mut scene_rect, |ui| {
                    for row in 0..20 {
                        ui.label(format!("Scene {row}"));
                    }
                });
            *scene_rect_clone.lock() = scene_rect;
            for row in 0..ROWS {
                ui.label(format!("Below {row}"));
            }
        });
        store(&o, &output);
    });

    // Press on the first scene label, near the top left, and hold well inside the viewport:
    // The scene is zoomed out to fit and centred, so its first label is near (20, 8) on screen:
    let inside = pos2(30.0, 60.0);
    press_and_hold(&mut harness, pos2(24.0, 11.0), inside, 20);
    let scrolled = offset(&outer);
    release(&mut harness, inside);
    assert!(
        copy(&mut harness).is_some(),
        "The drag should have selected some scene text"
    );
    assert_eq!(
        scrolled,
        Vec2::ZERO,
        "The pointer is inside the viewport on screen, so nothing should scroll"
    );
}

#[test]
fn the_selection_follows_back_past_its_anchor() {
    let (mut harness, state) = label_harness();

    // Drag down from the first row until well scrolled:
    let start = pos2(20.0, 30.0);
    press_and_hold(&mut harness, start, pos2(20.0, SIZE.y + 30.0), 30);
    assert!(offset(&state).y > SIZE.y, "Should have scrolled well down");

    // Then hold the pointer above the area until it is back at the top:
    let above = pos2(20.0, -30.0);
    harness.hover_at(above);
    harness.run_steps(60);
    assert_eq!(offset(&state).y, 0.0, "Should be back at the top");

    release(&mut harness, above);
    let copied = copy(&mut harness).expect("Should have a selection to copy");
    // The drag started in the second row, so only the rows above it are left:
    assert!(
        !copied.contains("Row 2"),
        "The selection should have shrunk back to the anchor, got {copied:?}"
    );
}

#[test]
fn a_vertical_area_leaves_sideways_scrolling_to_an_outer_area() {
    let outer: SharedState = Default::default();
    let o = Arc::clone(&outer);
    let mut harness = harness_with(move |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let output = ScrollArea::horizontal().auto_shrink(false).show(ui, |ui| {
            ui.set_min_width(4.0 * SIZE.x);
            ScrollArea::vertical()
                .id_salt("inner")
                .max_width(SIZE.x / 2.0)
                .auto_shrink(false)
                .show(ui, |ui| {
                    for row in 0..ROWS {
                        ui.label(format!("Row {row}"));
                    }
                });
        });
        store(&o, &output);
    });

    // Drag from a row of the inner, vertical-only area out past the window's right edge:
    press_and_hold(
        &mut harness,
        pos2(10.0, 30.0),
        pos2(SIZE.x + 20.0, 30.0),
        20,
    );
    assert!(
        offset(&outer).x > 0.0,
        "The inner area cannot scroll sideways, so the outer one should, got {}",
        offset(&outer).x
    );
}

#[test]
fn the_selection_follows_back_past_its_anchor_upwards_then_down() {
    let (mut harness, state) = label_harness();

    // Start in the middle of the list:
    harness.hover_at(pos2(100.0, 50.0));
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: Vec2::new(0.0, -400.0),
        modifiers: Modifiers::NONE,
        phase: TouchPhase::Move,
        source: MouseWheelSource::Wheel,
    });
    harness.run_steps(30);
    let start_offset = offset(&state).y;
    assert!(start_offset > 0.0);

    // Drag up until at the top, then hold below until past where we started:
    press_and_hold(&mut harness, pos2(20.0, 40.0), pos2(20.0, -30.0), 60);
    assert_eq!(offset(&state).y, 0.0, "Should be at the top");
    let below = pos2(20.0, SIZE.y + 30.0);
    harness.hover_at(below);
    harness.run_steps(60);
    assert!(offset(&state).y > start_offset, "Should be past the start");

    release(&mut harness, below);
    let copied = copy(&mut harness).expect("Should have a selection to copy");
    assert!(
        !copied.contains("Row 0"),
        "The selection should have shrunk back from the top to the anchor, got {copied:?}"
    );
}

#[test]
fn a_horizontal_area_in_a_vertical_page_scrolls_the_page() {
    // Like a code block in a document.
    let page: SharedState = Default::default();
    let p = Arc::clone(&page);
    let mut harness = harness_with(move |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let output = ScrollArea::vertical()
            .id_salt("page")
            .auto_shrink(false)
            .show(ui, |ui| {
                ScrollArea::horizontal().id_salt("code").show(ui, |ui| {
                    for row in 0..10 {
                        ui.label(format!("Code {row}"));
                    }
                });
                for row in 0..ROWS {
                    ui.label(format!("Text {row}"));
                }
            });
        store(&p, &output);
    });

    let below = pos2(20.0, SIZE.y + 20.0);
    press_and_hold(&mut harness, pos2(20.0, 12.0), below, 20);
    assert!(
        offset(&page).y > 0.0,
        "The code block cannot scroll down, so the page should, got {}",
        offset(&page).y
    );
}

#[test]
fn a_nested_area_almost_scrolled_out_of_view_does_not_panic() {
    let outer: SharedState = Default::default();
    let inner: SharedState = Default::default();
    let (o, i) = (Arc::clone(&outer), Arc::clone(&inner));
    let space = Arc::new(Mutex::new(50.0_f32));
    let space_clone = Arc::clone(&space);
    let mut harness = harness_with(move |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let output = ScrollArea::vertical()
            .id_salt("outer")
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.add_space(*space_clone.lock());
                let output = ScrollArea::vertical()
                    .id_salt("inner")
                    .max_height(40.0)
                    .show(ui, |ui| {
                        for row in 0..20 {
                            ui.label(format!("Inner {row}"));
                        }
                    });
                store(&i, &output);
                for row in 0..ROWS {
                    ui.label(format!("Bottom {row}"));
                }
            });
        store(&o, &output);
    });

    // Move the nested area down until only 1.5 pt of it shows at the bottom of the outer one:
    let outer_viewport = viewport(&outer);
    *space.lock() += (outer_viewport.bottom() - 1.5) - viewport(&inner).top();
    harness.run_steps(2);

    // Press on that sliver and drag below the window:
    let below = pos2(15.0, SIZE.y + 20.0);
    press_and_hold(
        &mut harness,
        pos2(15.0, outer_viewport.bottom() - 0.75),
        below,
        5,
    );
    release(&mut harness, below);
}

#[test]
fn the_selection_reaches_labels_below_a_scroll_area() {
    use egui_kittest::kittest::Queryable as _;

    for rows in [3, ROWS] {
        let inner: SharedState = Default::default();
        let i = Arc::clone(&inner);
        let mut harness = Harness::builder()
            .with_size(Vec2::new(200.0, 200.0))
            .with_step_dt(1.0 / 60.0)
            .build_ui_state(
                move |ui, _| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    let output = ScrollArea::vertical().max_height(40.0).show(ui, |ui| {
                        for row in 0..rows {
                            ui.label(format!("Inner {row}"));
                        }
                    });
                    store(&i, &output);
                    for row in 0..4 {
                        ui.label(format!("Below {row}"));
                    }
                },
                String::new(),
            );
        harness.run_steps(2);

        // From the first inner row to the last label below the scroll area:
        let last = harness.get_by_label("Below 3").rect();
        let end = pos2(last.left() + 10.0, last.center().y);
        press_and_hold(&mut harness, pos2(10.0, 12.0), end, 150);
        if rows == 3 {
            assert_eq!(offset(&inner), Vec2::ZERO, "Nothing to scroll");
        } else {
            assert!(
                offset(&inner).y > 0.0,
                "Should first have scrolled the area to its end"
            );
        }
        release(&mut harness, end);
        let copied = copy(&mut harness).expect("Should have a selection to copy");
        assert!(
            copied.contains("Below 2"),
            "The selection should reach the labels below (rows={rows}), got {copied:?}"
        );
    }
}

#[test]
fn a_nested_area_that_cannot_scroll_further_leaves_it_to_the_page() {
    for (rows, frames) in [(3, 30), (ROWS, 150)] {
        let inner: SharedState = Default::default();
        let page: SharedState = Default::default();
        let (i, p) = (Arc::clone(&inner), Arc::clone(&page));
        let mut harness = harness_with(move |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let output = ScrollArea::vertical()
                .id_salt("page")
                .auto_shrink(false)
                .show(ui, |ui| {
                    let output = ScrollArea::vertical()
                        .id_salt("inner")
                        .max_height(40.0)
                        .show(ui, |ui| {
                            for row in 0..rows {
                                ui.label(format!("Inner {row}"));
                            }
                        });
                    store(&i, &output);
                    for row in 0..ROWS {
                        ui.label(format!("Bottom {row}"));
                    }
                });
            store(&p, &output);
        });

        press_and_hold(
            &mut harness,
            pos2(10.0, 12.0),
            pos2(30.0, SIZE.y + 20.0),
            frames,
        );
        assert!(
            offset(&page).y > 0.0,
            "Once the nested area is at its end, the page should scroll (rows={rows}), got {}",
            offset(&page).y
        );
    }
}

#[test]
fn the_selection_reaches_a_label_below_a_scroll_area_in_a_window() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 300.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            |ui, _| {
                egui::Window::new("Window")
                    .fixed_pos(Pos2::ZERO)
                    .show(ui.ctx(), |ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        ScrollArea::vertical().max_height(60.0).show(ui, |ui| {
                            for row in 0..ROWS {
                                ui.label(format!("Inner {row}"));
                            }
                        });
                        ui.label("Footer");
                    });
            },
            String::new(),
        );
    harness.run_steps(2);

    // Hold below the window, past the footer:
    let below = pos2(30.0, 250.0);
    press_and_hold(&mut harness, pos2(10.0, 45.0), below, 120);
    release(&mut harness, below);
    let copied = copy(&mut harness).expect("Should have a selection to copy");
    assert!(
        copied.contains("Footer"),
        "The selection should reach the label below the scroll area, got {copied:?}"
    );
}

#[test]
fn only_the_axis_the_pointer_is_past_is_scrolled() {
    let outer: SharedState = Default::default();
    let inner: SharedState = Default::default();
    let (o, i) = (Arc::clone(&outer), Arc::clone(&inner));
    let mut harness = harness_with(move |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let output = ScrollArea::horizontal().auto_shrink(false).show(ui, |ui| {
            ui.set_min_width(4.0 * SIZE.x);
            let output = ScrollArea::vertical()
                .id_salt("inner")
                .max_width(SIZE.x / 2.0)
                .auto_shrink(false)
                .show(ui, |ui| {
                    for row in 0..ROWS {
                        ui.label(format!("Row {row}"));
                    }
                });
            store(&i, &output);
        });
        store(&o, &output);
    });

    // Outside the inner area to the right, but inside the outer one, and inside both vertically:
    press_and_hold(&mut harness, pos2(10.0, 30.0), pos2(150.0, 30.0), 20);
    assert_eq!(offset(&outer), Vec2::ZERO, "Inside the outer viewport");
    assert_eq!(
        offset(&inner),
        Vec2::ZERO,
        "Inside the inner viewport vertically"
    );
}

#[test]
fn scrolls_with_the_pointer_just_past_the_edge() {
    for past_edge in [1.0, 3.0] {
        let (mut harness, state) = label_harness();
        let below = pos2(20.0, viewport(&state).bottom() + past_edge);
        press_and_hold(&mut harness, pos2(20.0, 30.0), below, 20);
        assert!(
            offset(&state).y > 0.0,
            "Should scroll with the pointer {past_edge} pt past the edge"
        );
    }
}

#[test]
fn a_label_in_a_window_below_the_area_does_not_stop_the_scrolling() {
    let state: SharedState = Default::default();
    let s = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 200.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, _| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let output = ScrollArea::vertical()
                    .max_height(100.0)
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        for row in 0..ROWS {
                            ui.label(format!("Row {row}"));
                        }
                    });
                store(&s, &output);
                // In another layer, so the selection cannot extend into it:
                egui::Window::new("Window")
                    .fixed_pos(pos2(0.0, 130.0))
                    .show(ui.ctx(), |ui| {
                        for row in 0..3 {
                            ui.label(format!("Window {row}"));
                        }
                    });
            },
            String::new(),
        );
    harness.run_steps(2);

    press_and_hold(&mut harness, pos2(20.0, 30.0), pos2(30.0, 165.0), 40);
    assert!(
        offset(&state).y > 0.0,
        "The window's labels are not part of the selection, so the area should scroll"
    );
}

/// With the default item spacing, so there are gaps between the labels below the area.
#[test]
fn the_area_keeps_scrolling_steadily_over_the_labels_below_it() {
    use egui_kittest::kittest::Queryable as _;

    for pointer_at in ["gap", "right of the text"] {
        let inner: SharedState = Default::default();
        let i = Arc::clone(&inner);
        let mut harness = Harness::builder()
            .with_size(Vec2::new(200.0, 200.0))
            .with_step_dt(1.0 / 60.0)
            .build_ui_state(
                move |ui, _| {
                    let output = ScrollArea::vertical().max_height(40.0).show(ui, |ui| {
                        for row in 0..ROWS {
                            ui.label(format!("Inner {row}"));
                        }
                    });
                    store(&i, &output);
                    for row in 0..4 {
                        ui.label(format!("Below {row}"));
                    }
                },
                String::new(),
            );
        harness.run_steps(2);

        let below_1 = harness.get_by_label("Below 1").rect();
        let below_2 = harness.get_by_label("Below 2").rect();
        let pointer = if pointer_at == "gap" {
            pos2(
                below_1.left() + 10.0,
                0.5 * (below_1.bottom() + below_2.top()),
            )
        } else {
            pos2(below_1.right() + 20.0, below_1.center().y)
        };
        press_and_hold(&mut harness, pos2(12.0, 12.0), pointer, 5);
        let mut last = offset(&inner).y;
        for _ in 0..10 {
            harness.step();
            let now = offset(&inner).y;
            assert!(
                last < now,
                "Should keep scrolling, with the pointer in the {pointer_at}: {last} -> {now}"
            );
            last = now;
        }
    }
}

#[test]
fn a_neighbouring_list_does_not_stop_the_scrolling() {
    let left: SharedState = Default::default();
    let right: SharedState = Default::default();
    let (l, r) = (Arc::clone(&left), Arc::clone(&right));
    let mut harness = harness_with(move |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        ui.columns(2, |columns| {
            let output = ScrollArea::vertical()
                .id_salt("left")
                .auto_shrink(false)
                .show(&mut columns[0], |ui| {
                    for row in 0..ROWS {
                        ui.label(format!("Left {row}"));
                    }
                });
            store(&l, &output);
            let output = ScrollArea::vertical()
                .id_salt("right")
                .max_height(40.0)
                .show(&mut columns[1], |ui| {
                    for row in 0..ROWS {
                        ui.label(format!("Right {row}"));
                    }
                });
            store(&r, &output);
        });
    });

    // Drag in the short right list, and hold straight below it, level with the left list:
    let start = pos2(viewport(&right).left() + 10.0, viewport(&right).top() + 5.0);
    let below = pos2(start.x, viewport(&right).bottom() + 30.0);
    press_and_hold(&mut harness, start, below, 10);
    let after_10 = offset(&right).y;
    harness.run_steps(10);
    assert!(
        offset(&right).y > after_10 && after_10 > 0.0,
        "The left list's labels are not below the right one, so it should keep scrolling, got {after_10} -> {}",
        offset(&right).y
    );
}

#[test]
fn the_selection_does_not_jump_on_release_after_auto_scrolling() {
    let state: SharedState = Default::default();
    let s = Arc::clone(&state);
    let mut harness = harness_with(move |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let output = ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            for row in 0..ROWS {
                ui.label(format!("Row {row} with some more words"));
            }
        });
        store(&s, &output);
    });

    let below = pos2(40.0, SIZE.y + 20.0);
    press_and_hold(&mut harness, pos2(20.0, 20.0), below, 20);
    let while_held = copy(&mut harness).expect("Should have a selection to copy");
    release(&mut harness, below);
    let after_release = reported_selection(&harness).expect("Should report the selection");
    // It has grown by a row or two meanwhile, but still ends at the pointer, mid-row:
    assert!(
        !while_held.ends_with("words") && !after_release.ends_with("words"),
        "The selection should still end where the pointer is, got {while_held:?} -> {after_release:?}"
    );
}

#[test]
fn a_short_area_beside_a_tall_one_still_scrolls() {
    let short: SharedState = Default::default();
    let s = Arc::clone(&short);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 200.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, _| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.horizontal_top(|ui| {
                    let output = ScrollArea::vertical()
                        .id_salt("short")
                        .max_height(40.0)
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                for row in 0..ROWS {
                                    ui.label(format!("Short {row}"));
                                }
                            });
                        });
                    store(&s, &output);
                    ScrollArea::vertical().id_salt("tall").show(ui, |ui| {
                        ui.vertical(|ui| {
                            for row in 0..ROWS {
                                ui.label(format!("Tall {row}"));
                            }
                        });
                    });
                });
            },
            String::new(),
        );
    harness.run_steps(2);

    // Drag in the short area, and hold straight below it:
    let area = viewport(&short);
    let below = pos2(area.left() + 12.0, area.bottom() + 30.0);
    press_and_hold(
        &mut harness,
        pos2(area.left() + 5.0, area.top() + 5.0),
        below,
        30,
    );
    assert!(
        offset(&short).y > 0.0,
        "The tall area's labels are beside, not below, so the short one should scroll"
    );
}

#[test]
fn moving_and_releasing_in_the_same_frame_does_not_jump_past_the_area() {
    let state: SharedState = Default::default();
    let s = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 200.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, _| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let output = ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                    for row in 0..ROWS {
                        ui.label(format!("Row {row} with some words"));
                    }
                });
                store(&s, &output);
                ui.add_space(20.0);
                ui.label("Footer label below the area");
            },
            String::new(),
        );
    harness.run_steps(2);

    // Auto-scroll down a little, with the pointer just below the area:
    let area = viewport(&state);
    press_and_hold(
        &mut harness,
        pos2(area.left() + 5.0, area.top() + 5.0),
        pos2(40.0, area.bottom() + 10.0),
        10,
    );
    // Then move onto the footer and release, in one frame
    // (`Harness::event` would queue the release for a frame of its own):
    let footer = pos2(100.0, area.bottom() + 27.0);
    let events = &mut harness.input_mut().events;
    events.push(Event::PointerMoved(footer));
    events.push(Event::PointerButton {
        pos: footer,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    let copied = copy(&mut harness).expect("Should have a selection to copy");
    assert!(
        !copied.contains("Footer label"),
        "The area could still scroll, so the selection should not jump to the footer, got {copied:?}"
    );
}

#[test]
fn moving_up_and_releasing_in_the_same_frame_does_not_jump_past_the_area() {
    let state: SharedState = Default::default();
    let s = Arc::clone(&state);
    let mut first_frame = true;
    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 200.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, _| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.label("Header label above the area");
                ui.add_space(20.0);
                let mut scroll_area = ScrollArea::vertical().max_height(100.0);
                if core::mem::take(&mut first_frame) {
                    scroll_area = scroll_area.vertical_scroll_offset(400.0);
                }
                let output = scroll_area.show(ui, |ui| {
                    for row in 0..ROWS {
                        ui.label(format!("Row {row} with some words"));
                    }
                });
                store(&s, &output);
            },
            String::new(),
        );
    harness.run_steps(2);

    // Auto-scroll up a little, with the pointer just above the area:
    let area = viewport(&state);
    press_and_hold(
        &mut harness,
        pos2(area.left() + 5.0, area.bottom() - 5.0),
        pos2(40.0, area.top() - 8.0),
        10,
    );
    // Then move onto the header (shown before the area) and release, in one frame:
    let header = pos2(30.0, 7.0);
    let events = &mut harness.input_mut().events;
    events.push(Event::PointerMoved(header));
    events.push(Event::PointerButton {
        pos: header,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    let copied = copy(&mut harness).expect("Should have a selection to copy");
    assert!(
        !copied.contains("label above the area"),
        "The area could still scroll, so the selection should not jump to the header, got {copied:?}"
    );
}

#[test]
fn a_selection_in_a_window_survives_scrolling_out_of_view() {
    let state: SharedState = Default::default();
    let s = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(400.0, 300.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, _| {
                egui::Window::new("Window")
                    .fixed_pos(pos2(20.0, 20.0))
                    .fixed_size(Vec2::new(200.0, 100.0))
                    .show(ui.ctx(), |ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        let output = ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                            for row in 0..ROWS {
                                ui.label(format!("Row {row}"));
                            }
                        });
                        store(&s, &output);
                    });
            },
            String::new(),
        );
    harness.run_steps(3);

    let area = viewport(&state);
    let end = pos2(area.left() + 10.0, area.top() + 20.0);
    press_and_hold(
        &mut harness,
        pos2(area.left() + 5.0, area.top() + 5.0),
        end,
        1,
    );
    release(&mut harness, end);
    let selected = copy(&mut harness).expect("Should have a selection to copy");

    harness.hover_at(area.center());
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: Vec2::new(0.0, -500.0),
        modifiers: Modifiers::NONE,
        phase: TouchPhase::Move,
        source: MouseWheelSource::Wheel,
    });
    harness.run_steps(30);
    assert!(
        offset(&state).y > 100.0,
        "Should have scrolled the selection out of view"
    );
    assert_eq!(copy(&mut harness), Some(selected));
}

/// Like the "Scrolling" demo's appearance tab: some labels and a slider, then a long text
/// in a scroll area, scrolled to its end.
fn labels_above_harness() -> (Harness<'static, String>, SharedState) {
    let state: SharedState = Default::default();
    let s = Arc::clone(&state);
    let mut first_frame = true;
    let mut harness = Harness::builder()
        .with_size(Vec2::new(500.0, 400.0))
        .with_step_dt(1.0 / 60.0)
        .with_accessibility_check(false) // the slider is unlabelled
        .build_ui_state(
            move |ui, _| {
                ui.label("Presets:");
                ui.weak("When to show scroll bars");
                ui.separator();
                let mut value = 21.0_f32;
                ui.add(egui::Slider::new(&mut value, 1.0..=100.0).text("Content length"));
                ui.separator();
                let mut scroll_area = ScrollArea::vertical().auto_shrink(false);
                if core::mem::take(&mut first_frame) {
                    scroll_area = scroll_area.vertical_scroll_offset(1.0e6);
                }
                let output = scroll_area.show(ui, |ui| {
                    for paragraph in 0..21 {
                        ui.label(format!(
                            "Paragraph {paragraph}: lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua."
                        ));
                    }
                });
                store(&s, &output);
            },
            String::new(),
        );
    harness.run_steps(3);
    (harness, state)
}

#[test]
fn the_area_scrolls_to_its_start_before_the_selection_extends_above_it() {
    let (mut harness, state) = labels_above_harness();
    assert!(offset(&state).y > 0.0, "Should start scrolled to the end");

    // From the bottom of the area up onto the slider's row, just above it:
    let area = viewport(&state);
    let on_the_slider = pos2(area.left() + 30.0, area.top() - 20.0);
    press_and_hold(
        &mut harness,
        pos2(area.left() + 30.0, area.bottom() - 8.0),
        on_the_slider,
        10,
    );
    assert!(
        offset(&state).y < 1.0e6,
        "Should be scrolling up towards the start"
    );
    let scrolled_a_bit = offset(&state).y;
    harness.run_steps(10);
    assert!(
        offset(&state).y < scrolled_a_bit,
        "Should keep scrolling up while there is more above"
    );
    let partly = copy(&mut harness).expect("Should have a selection to copy");
    assert!(
        !partly.contains("Content length"),
        "The selection should not reach above the area before it is scrolled to its start, got {partly:?}"
    );

    // Hold on until the area is at its start; then the selection extends to the labels above:
    harness.run_steps(120);
    assert_eq!(offset(&state).y, 0.0, "Should be at the start");
    let on_the_top_label = pos2(10.0, 10.0);
    harness.hover_at(on_the_top_label);
    harness.run_steps(2);
    release(&mut harness, on_the_top_label);
    let copied = copy(&mut harness).expect("Should have a selection to copy");
    assert!(
        copied.contains("Content length"),
        "Once at the start, the selection should extend above the area, got {copied:?}"
    );
}

#[test]
fn copied_text_breaks_the_line_between_text_above_and_in_a_scrolled_area() {
    let (mut harness, state) = labels_above_harness();

    // Select from the bottom of the area up to the top label, with the area left scrolled to its
    // end (shift-click), so that the paragraphs scrolled out of view sit above the area:
    let area = viewport(&state);
    let bottom = pos2(area.left() + 30.0, area.bottom() - 8.0);
    harness.hover_at(bottom);
    harness.step();
    harness.drag_at(bottom);
    harness.step();
    release(&mut harness, bottom);
    let top = pos2(10.0, 10.0);
    harness.hover_at(top);
    harness.step();
    harness.event_modifiers(
        Event::PointerButton {
            pos: top,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::SHIFT,
        },
        Modifiers::SHIFT,
    );
    harness.step();
    harness.event_modifiers(
        Event::PointerButton {
            pos: top,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::SHIFT,
        },
        Modifiers::SHIFT,
    );
    harness.step();
    assert!(
        offset(&state).y > 0.0,
        "Should still be scrolled to the end"
    );

    let copied = copy(&mut harness).expect("Should have a selection to copy");
    assert!(
        copied.contains("Content length\n") && copied.contains("\nParagraph 0:"),
        "The first paragraph of the area should start on a line of its own, got {copied:?}"
    );
}

/// Like the "Scrolling" demo's many-lines tab: a list that only lays out the visible rows.
fn virtual_list_harness() -> (Harness<'static, String>, SharedState) {
    let state: SharedState = Default::default();
    let s = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 200.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, _| {
                ui.label("A lot of rows, but only the visible ones are laid out:");
                let row_height = ui.text_style_height(&egui::TextStyle::Body);
                let output = ScrollArea::vertical().auto_shrink(false).show_rows(
                    ui,
                    row_height,
                    10_000,
                    |ui, rows| {
                        for row in rows {
                            ui.label(format!("This is row {}", row + 1));
                        }
                    },
                );
                store(&s, &output);
            },
            String::new(),
        );
    harness.run_steps(2);
    (harness, state)
}

#[test]
fn a_list_that_lays_out_only_visible_rows_does_not_auto_scroll() {
    // The rows scrolled out of view would not be laid out, so neither selected nor copied.
    let (mut harness, state) = virtual_list_harness();
    let area = viewport(&state);
    let below = pos2(20.0, area.bottom() + 20.0);
    press_and_hold(&mut harness, pos2(20.0, area.top() + 20.0), below, 30);
    assert_eq!(offset(&state), Vec2::ZERO, "Should not scroll");
    release(&mut harness, below);
    assert!(
        copy(&mut harness).is_some(),
        "The selection should stay, up to the last visible row"
    );
}

#[test]
fn a_list_that_lays_out_only_visible_rows_does_not_auto_scroll_from_above_either() {
    use egui_kittest::kittest::Queryable as _;

    let (mut harness, state) = virtual_list_harness();
    let area = viewport(&state);
    let heading = harness
        .get_by_label("A lot of rows, but only the visible ones are laid out:")
        .rect();
    let below = pos2(20.0, area.bottom() + 20.0);
    press_and_hold(
        &mut harness,
        pos2(heading.left() + 2.0, heading.center().y),
        below,
        30,
    );
    assert_eq!(offset(&state), Vec2::ZERO, "Should not scroll");
}

#[test]
fn starting_above_a_list_scrolls_the_list_the_selection_moves_into() {
    use egui_kittest::kittest::Queryable as _;

    let state: SharedState = Default::default();
    let s = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 200.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, _| {
                ui.label("Heading above the list");
                let output = ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    for row in 0..ROWS {
                        ui.label(format!("Row {row}"));
                    }
                });
                store(&s, &output);
            },
            String::new(),
        );
    harness.run_steps(2);

    // From the heading, which is in no scroll area, down past the list's bottom edge:
    let heading = harness.get_by_label("Heading above the list").rect();
    let area = viewport(&state);
    let below = pos2(20.0, area.bottom() + 20.0);
    press_and_hold(
        &mut harness,
        pos2(heading.left() + 2.0, heading.center().y),
        below,
        30,
    );
    assert!(
        offset(&state).y > 0.0,
        "The list the selection has moved into should scroll"
    );
    release(&mut harness, below);
    let copied = copy(&mut harness).expect("Should have a selection to copy");
    assert!(
        copied.contains("Heading") && copied.contains("Row 20"),
        "The selection should run from the heading into rows that were out of view, got {copied:?}"
    );
}

/// Step like a reactive integration: only while a repaint is requested. Returns the steps taken.
fn run_reactive(harness: &mut Harness<'_, String>, max_steps: usize) -> usize {
    for steps in 0..max_steps {
        if !harness.ctx.has_requested_repaint() {
            return steps;
        }
        harness.step();
    }
    max_steps
}

#[test]
fn starting_below_a_list_scrolls_it_up_to_its_start() {
    let state: SharedState = Default::default();
    let s = Arc::clone(&state);
    let mut first_frame = true;
    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 150.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, _| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let mut scroll_area = ScrollArea::vertical().max_height(100.0).auto_shrink(false);
                if core::mem::take(&mut first_frame) {
                    scroll_area = scroll_area.vertical_scroll_offset(1.0e6);
                }
                let output = scroll_area.show(ui, |ui| {
                    for row in 0..ROWS {
                        ui.label(format!("Row {row}"));
                    }
                });
                store(&s, &output);
                ui.label("Footer below the list");
            },
            String::new(),
        );
    harness.run_steps(2);
    assert!(offset(&state).y > 0.0, "Should start scrolled to the end");

    // From the footer, which is in no scroll area, up past the list's top edge:
    let area = viewport(&state);
    let footer = pos2(10.0, area.bottom() + 6.0);
    harness.hover_at(footer);
    harness.step();
    harness.drag_at(footer);
    harness.step();
    harness.hover_at(pos2(30.0, area.top() - 3.0));
    harness.step();
    run_reactive(&mut harness, 600);
    assert_eq!(
        offset(&state).y,
        0.0,
        "Holding the pointer above the list should scroll it all the way up"
    );
}

#[test]
fn a_table_that_lays_out_only_visible_rows_does_not_auto_scroll() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 150.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            |ui, _| {
                egui_extras::TableBuilder::new(ui)
                    .column(egui_extras::Column::remainder())
                    .body(|body| {
                        body.rows(18.0, 200, |mut row| {
                            let index = row.index();
                            row.col(|ui| {
                                ui.label(format!("Row {index}"));
                            });
                        });
                    });
            },
            String::new(),
        );
    harness.run_steps(2);

    let below = pos2(30.0, 180.0);
    press_and_hold(&mut harness, pos2(10.0, 9.0), below, 60);
    release(&mut harness, below);
    let copied = copy(&mut harness).expect("The selection should stay");
    assert!(copied.contains("Row 0"), "got {copied:?}");
}

#[test]
fn copying_a_row_of_labels_scrolled_sideways_keeps_it_on_one_line() {
    let state: SharedState = Default::default();
    let s = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 60.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, _| {
                let output = ScrollArea::horizontal().auto_shrink(false).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for word in 0..30 {
                            ui.label(format!("w{word}"));
                        }
                    });
                });
                store(&s, &output);
            },
            String::new(),
        );
    harness.run_steps(2);

    let right = pos2(230.0, 15.0);
    press_and_hold(&mut harness, pos2(10.0, 15.0), right, 40);
    assert!(offset(&state).x > 0.0, "Should have scrolled sideways");
    release(&mut harness, right);
    let copied = copy(&mut harness).expect("Should have a selection to copy");
    assert!(
        !copied.contains('\n'),
        "One row of labels should copy as one line, got {copied:?}"
    );
}

#[test]
fn copying_paragraphs_scrolled_out_of_view_keeps_the_blank_lines() {
    let state: SharedState = Default::default();
    let s = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 150.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, _| {
                let output = ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    for paragraph in 0..30 {
                        ui.label(format!("P{paragraph}"));
                        ui.add_space(20.0);
                    }
                });
                store(&s, &output);
            },
            String::new(),
        );
    harness.run_steps(2);

    let below = pos2(30.0, 170.0);
    press_and_hold(&mut harness, pos2(10.0, 15.0), below, 25);
    assert!(offset(&state).y > 0.0, "Should have scrolled");
    release(&mut harness, below);
    let copied = copy(&mut harness).expect("Should have a selection to copy");
    let lines: Vec<&str> = copied.lines().collect();
    for pair in lines.windows(2) {
        assert!(
            pair[0].is_empty() || pair[1].is_empty(),
            "Paragraphs with space between them should be separated by blank lines, got {copied:?}"
        );
    }
}

#[test]
fn a_table_inside_a_scrolling_page_keeps_the_selection() {
    // Scrolling the page would scroll the table's rows out of view, and so out of the layout.
    for table_scrolls in [false, true] {
        let page: SharedState = Default::default();
        let p = Arc::clone(&page);
        let mut harness = Harness::builder()
            .with_size(Vec2::new(200.0, 150.0))
            .with_step_dt(1.0 / 60.0)
            .build_ui_state(
                move |ui, _| {
                    let output = ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                        ui.label("Above the table");
                        egui_extras::TableBuilder::new(ui)
                            .vscroll(table_scrolls)
                            .max_scroll_height(1.0e6)
                            .column(egui_extras::Column::remainder())
                            .body(|body| {
                                body.rows(18.0, 100, |mut row| {
                                    let index = row.index();
                                    row.col(|ui| {
                                        ui.label(format!("Row {index}"));
                                    });
                                });
                            });
                        ui.label("Below the table");
                    });
                    store(&p, &output);
                },
                String::new(),
            );
        harness.run_steps(2);

        let below = pos2(30.0, viewport(&page).bottom() + 10.0);
        press_and_hold(&mut harness, pos2(20.0, 45.0), below, 30);
        assert_eq!(
            offset(&page),
            Vec2::ZERO,
            "The page should not scroll the table's rows away (table scrolls: {table_scrolls})"
        );
        release(&mut harness, below);
        let copied = copy(&mut harness);
        assert!(
            copied.as_deref().is_some_and(|text| text.contains("Row")),
            "The selection should stay (table scrolls: {table_scrolls}), got {copied:?}"
        );
    }
}

#[test]
fn a_table_that_does_not_scroll_leaves_scrolling_to_the_list_the_selection_moves_into() {
    use egui_kittest::kittest::Queryable as _;

    let list: SharedState = Default::default();
    let l = Arc::clone(&list);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 150.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, _| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.columns(2, |columns| {
                    egui_extras::TableBuilder::new(&mut columns[0])
                        .vscroll(false)
                        .column(egui_extras::Column::remainder())
                        .body(|body| {
                            body.rows(18.0, 3, |mut row| {
                                let index = row.index();
                                row.col(|ui| {
                                    ui.label(format!("Left {index}"));
                                });
                            });
                        });
                    let output =
                        ScrollArea::vertical()
                            .max_height(100.0)
                            .show(&mut columns[1], |ui| {
                                for row in 0..ROWS {
                                    ui.label(format!("Right {row}"));
                                }
                            });
                    store(&l, &output);
                });
            },
            String::new(),
        );
    harness.run_steps(2);

    // From a row of the table, which scrolls along neither axis, into the list and below it:
    let first_row = harness.get_by_label("Left 0").rect();
    let area = viewport(&list);
    press_and_hold(
        &mut harness,
        pos2(first_row.left() + 2.0, first_row.center().y),
        area.center(),
        2,
    );
    harness.hover_at(pos2(area.left() + 10.0, area.bottom() + 10.0));
    harness.run_steps(40);
    assert!(
        offset(&list).y > 0.0,
        "The list the selection has moved into should scroll"
    );
}

#[test]
fn an_area_with_nothing_to_scroll_leaves_scrolling_to_the_list_the_selection_moves_into() {
    use egui_kittest::kittest::Queryable as _;

    #[derive(Clone, Copy, Debug)]
    enum Left {
        /// A table whose rows fit
        Table,

        /// A scroll area whose content fits
        Fits,

        /// A scroll area whose content is a fraction of a point too tall to fit
        JustOverflows,
    }

    for left in [Left::Table, Left::Fits, Left::JustOverflows] {
        let list: SharedState = Default::default();
        let l = Arc::clone(&list);
        let left_max_offset = Arc::new(Mutex::new(None));
        let m = Arc::clone(&left_max_offset);
        let mut harness = Harness::builder()
            .with_size(Vec2::new(300.0, 150.0))
            .with_step_dt(1.0 / 60.0)
            .build_ui_state(
                move |ui, _| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    ui.columns(2, |columns| {
                        // Able to scroll, but its content fits:
                        match left {
                            Left::Table => {
                                egui_extras::TableBuilder::new(&mut columns[0])
                                    .column(egui_extras::Column::remainder())
                                    .body(|body| {
                                        body.rows(18.0, 3, |mut row| {
                                            let index = row.index();
                                            row.col(|ui| {
                                                ui.label(format!("Left {index}"));
                                            });
                                        });
                                    });
                            }
                            Left::Fits | Left::JustOverflows => {
                                let (rows, max_height) = match left {
                                    Left::JustOverflows => (6, 6.0 * 15.0 - 0.3),
                                    _ => (3, f32::INFINITY),
                                };
                                let output = ScrollArea::vertical()
                                    .id_salt("left")
                                    .max_height(max_height)
                                    .show(&mut columns[0], |ui| {
                                        for row in 0..rows {
                                            ui.label(format!("Left {row}"));
                                        }
                                    });
                                *m.lock() =
                                    Some(output.content_size.y - output.inner_rect.height());
                            }
                        }
                        let output = ScrollArea::vertical()
                            .id_salt("right")
                            .max_height(100.0)
                            .show(&mut columns[1], |ui| {
                                for row in 0..ROWS {
                                    ui.label(format!("Right {row}"));
                                }
                            });
                        store(&l, &output);
                    });
                },
                String::new(),
            );
        harness.run_steps(2);
        if matches!(left, Left::JustOverflows) {
            let max_offset = left_max_offset.lock().expect("left area should be shown");
            assert!(
                0.0 < max_offset && max_offset < 0.5,
                "The left area should overflow by a fraction of a point, got {max_offset}"
            );
        }

        // From the left, into the list and below it:
        let first_row = harness.get_by_label("Left 0").rect();
        let area = viewport(&list);
        press_and_hold(
            &mut harness,
            pos2(first_row.left() + 2.0, first_row.center().y),
            area.center(),
            2,
        );
        let below = pos2(area.left() + 10.0, area.bottom() + 10.0);
        harness.hover_at(below);
        harness.run_steps(40);
        assert!(
            offset(&list).y > 0.0,
            "The list the selection has moved into should scroll ({left:?})"
        );
        release(&mut harness, below);
        let copied = copy(&mut harness).expect("Should have a selection to copy");
        assert!(
            copied.contains("Left 0") && copied.contains("Right 20"),
            "The selection should run from the left into rows that were out of view ({left:?}), got {copied:?}"
        );
    }
}

#[test]
fn an_area_scrolling_only_sideways_leaves_scrolling_down_to_the_list_below() {
    use egui_kittest::kittest::Queryable as _;

    let list: SharedState = Default::default();
    let l = Arc::clone(&list);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 200.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, _| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ScrollArea::horizontal().id_salt("code").show(ui, |ui| {
                    // Tall enough that the scroll bar does not cover the first line
                    for line in 0..3 {
                        ui.add(
                            egui::Label::new(format!("Code {line} {}", "wide ".repeat(40)))
                                .extend(),
                        );
                    }
                });
                let output = ScrollArea::vertical()
                    .id_salt("list")
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        for row in 0..ROWS {
                            ui.label(format!("Row {row}"));
                        }
                    });
                store(&l, &output);
            },
            String::new(),
        );
    harness.run_steps(2);

    // From the code, which only scrolls sideways, down past the list's bottom edge:
    let code = harness.get_by_label_contains("Code 0").rect();
    let area = viewport(&list);
    let below = pos2(20.0, area.bottom() + 20.0);
    press_and_hold(
        &mut harness,
        pos2(code.left() + 2.0, code.center().y),
        below,
        30,
    );
    assert!(
        offset(&list).y > 0.0,
        "The list the selection has moved into should scroll"
    );
    release(&mut harness, below);
    let copied = copy(&mut harness).expect("Should have a selection to copy");
    assert!(
        copied.contains("Code 0") && copied.contains("Row 20"),
        "The selection should run from the code into rows that were out of view, got {copied:?}"
    );
}

#[test]
fn an_area_scrolled_to_its_end_leaves_scrolling_to_the_list_the_selection_moves_into() {
    use egui_kittest::kittest::Queryable as _;

    let list: SharedState = Default::default();
    let l = Arc::clone(&list);
    let mut first_frame = true;
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 150.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, _| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.columns(2, |columns| {
                    let mut left = ScrollArea::vertical().id_salt("left").max_height(100.0);
                    if core::mem::take(&mut first_frame) {
                        left = left.vertical_scroll_offset(1.0e6);
                    }
                    left.show(&mut columns[0], |ui| {
                        for row in 0..ROWS {
                            ui.label(format!("Left {row}"));
                        }
                    });
                    let output = ScrollArea::vertical()
                        .id_salt("right")
                        .max_height(100.0)
                        .show(&mut columns[1], |ui| {
                            for row in 0..ROWS {
                                ui.label(format!("Right {row}"));
                            }
                        });
                    store(&l, &output);
                });
            },
            String::new(),
        );
    harness.run_steps(2);

    // From the last row on the left, which is scrolled to its end, into the list and below it:
    let last_row = harness.get_by_label(&format!("Left {}", ROWS - 1)).rect();
    let area = viewport(&list);
    press_and_hold(
        &mut harness,
        pos2(last_row.left() + 2.0, last_row.center().y),
        area.center(),
        2,
    );
    let below = pos2(area.left() + 10.0, area.bottom() + 10.0);
    harness.hover_at(below);
    harness.run_steps(40);
    assert!(
        offset(&list).y > 0.0,
        "The list the selection has moved into should scroll"
    );
    release(&mut harness, below);
    let copied = copy(&mut harness).expect("Should have a selection to copy");
    assert!(
        copied.contains(&format!("Left {}", ROWS - 1)) && copied.contains("Right 20"),
        "The selection should run from the left into rows that were out of view, got {copied:?}"
    );
}

#[test]
fn a_table_inside_a_scrolling_page_keeps_the_selection_moved_out_of_it() {
    use egui_kittest::kittest::Queryable as _;

    let page: SharedState = Default::default();
    let p = Arc::clone(&page);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 200.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, _| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let output = ScrollArea::vertical()
                    .id_salt("page")
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        egui_extras::TableBuilder::new(ui)
                            .max_scroll_height(60.0)
                            .column(egui_extras::Column::remainder())
                            .body(|body| {
                                body.rows(15.0, 5, |mut row| {
                                    let index = row.index();
                                    row.col(|ui| {
                                        ui.label(format!("Cell {index}"));
                                    });
                                });
                            });
                        for row in 0..50 {
                            ui.label(format!("After {row}"));
                        }
                    });
                store(&p, &output);
            },
            String::new(),
        );
    harness.run_steps(2);

    // From the first cell, out of the table and below the page:
    let cell = harness.get_by_label("Cell 0").rect();
    let area = viewport(&page);
    let below = pos2(20.0, area.bottom() + 10.0);
    press_and_hold(
        &mut harness,
        pos2(cell.left() + 2.0, cell.center().y),
        below,
        30,
    );
    assert_eq!(
        offset(&page).y,
        0.0,
        "The page should not scroll the table, which lays out only its visible rows, out of view"
    );
    release(&mut harness, below);
    let copied = copy(&mut harness).expect("Should have a selection to copy");
    assert!(
        copied.contains("Cell 0") && copied.contains("After 0"),
        "The selection should run from the table onto the page, got {copied:?}"
    );
}

#[test]
fn copying_from_a_nested_list_scrolled_out_of_view_breaks_the_line_after_it() {
    use egui_kittest::kittest::Queryable as _;

    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 200.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, _| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ScrollArea::vertical()
                    .id_salt("page")
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        ui.label("Heading");
                        ScrollArea::vertical()
                            .id_salt("list")
                            .max_height(60.0)
                            .show(ui, |ui| {
                                for row in 0..40 {
                                    ui.label(format!("Row {row}"));
                                }
                            });
                        for row in 0..50 {
                            ui.label(format!("After {row}"));
                        }
                    });
            },
            String::new(),
        );
    harness.run_steps(2);

    // From the heading, past the nested list, and below the page until both are out of view:
    let heading = harness.get_by_label("Heading").rect();
    let below = pos2(20.0, 210.0);
    press_and_hold(
        &mut harness,
        pos2(heading.left() + 2.0, heading.center().y),
        below,
        60,
    );
    release(&mut harness, below);
    let copied = copy(&mut harness).expect("Should have a selection to copy");
    assert!(
        copied.contains("Row 39") && copied.contains("After 0"),
        "The selection should run through the list onto the page, got {copied:?}"
    );
    assert!(
        !copied
            .lines()
            .any(|line| line.contains("Row") && line.contains("After")),
        "Each label should be on a line of its own, got {copied:?}"
    );
}

#[test]
fn copying_rows_with_sideways_scrolling_cells_out_of_view_keeps_each_row_on_one_line() {
    use egui_kittest::kittest::Queryable as _;

    let page: SharedState = Default::default();
    let p = Arc::clone(&page);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 200.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, _| {
                let output = ScrollArea::vertical()
                    .id_salt("page")
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        for row in 0..3 {
                            ui.horizontal(|ui| {
                                ui.label(format!("Name{row}:"));
                                ScrollArea::horizontal().id_salt(row).show(ui, |ui| {
                                    ui.label(format!("value{row}"));
                                });
                            });
                        }
                        for row in 0..50 {
                            ui.label(format!("After {row}"));
                        }
                    });
                store(&p, &output);
            },
            String::new(),
        );
    harness.run_steps(2);

    // Select from the first row into the labels below the rows, all in view:
    let name = harness.get_by_label("Name0:").rect();
    let after = harness.get_by_label("After 1").rect();
    let end = pos2(after.left() + 2.0, after.center().y);
    press_and_hold(
        &mut harness,
        pos2(name.left() + 2.0, name.center().y),
        end,
        2,
    );
    release(&mut harness, end);
    let in_view = copy(&mut harness).expect("Should have a selection to copy");
    assert!(
        in_view.starts_with("Name0: value0\n"),
        "Each row should copy as one line, got {in_view:?}"
    );

    // Scroll the rows out of view with the mouse wheel:
    harness.hover_at(pos2(100.0, 150.0));
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: Vec2::new(0.0, -300.0),
        modifiers: Modifiers::NONE,
        phase: TouchPhase::Move,
        source: MouseWheelSource::Wheel,
    });
    harness.run_steps(30);
    assert!(
        offset(&page).y > 100.0,
        "Should have scrolled the rows out of view"
    );
    let out_of_view = copy(&mut harness).expect("Should still have a selection to copy");
    assert_eq!(out_of_view, in_view);
}
