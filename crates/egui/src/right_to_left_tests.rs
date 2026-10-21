//! [`crate::Style::right_to_left`] for widget chrome, in both directions.

use core::cell::Cell;

use crate::{
    Button, Checkbox, CollapsingHeader, ComboBox, Context, Event, Id, Modifiers, PointerButton,
    Popup, Pos2, RadioButton, RawInput, Rect, Resize, Ui, Window,
    collapsing_header::CollapsingState,
    epaint::{ClippedShape, Shape},
    vec2,
};

fn context(right_to_left: bool) -> Context {
    let ctx = Context::default();
    ctx.all_styles_mut(|style| style.right_to_left = right_to_left);
    ctx
}

/// Runs one frame and returns what it painted.
fn run(ctx: &Context, events: Vec<Event>, mut add: impl FnMut(&mut Ui)) -> Vec<ClippedShape> {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| add(ui));
    output.textures_delta.clear();
    output.shapes
}

/// Runs a few frames, so popups and animations settle, and returns what the last one painted.
fn run_frames(ctx: &Context, add: impl FnMut(&mut Ui)) -> Vec<ClippedShape> {
    let add = core::cell::RefCell::new(add);
    for _ in 0..4 {
        run(ctx, vec![], |ui| (add.borrow_mut())(ui));
    }
    run(ctx, vec![], |ui| (add.borrow_mut())(ui))
}

/// The bounding rect of the text shape showing `text`.
fn text_rect(shapes: &[ClippedShape], text: &str) -> Rect {
    shapes
        .iter()
        .find(|clipped| matches!(&clipped.shape, Shape::Text(t) if t.galley.text() == text))
        .unwrap_or_else(|| panic!("No text {text:?}"))
        .shape
        .visual_bounding_rect()
}

/// The bounding rect of all non-text shapes inside `rect`.
fn non_text_rect(shapes: &[ClippedShape], rect: Rect) -> Rect {
    let bounds = shapes
        .iter()
        .filter(|clipped| !matches!(clipped.shape, Shape::Text(_)))
        .map(|clipped| clipped.shape.visual_bounding_rect())
        .filter(|bounds| bounds.is_positive() && rect.contains_rect(*bounds))
        .fold(Rect::NOTHING, |a, b| a.union(b));
    assert!(bounds.is_positive(), "Nothing painted in {rect:?}");
    bounds
}

/// The triangles (three-point paths) painted inside `rect`.
fn triangles(shapes: &[ClippedShape], rect: Rect) -> Vec<Vec<Pos2>> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Path(path)
                if path.points.len() == 3 && path.points.iter().all(|p| rect.contains(*p)) =>
            {
                Some(path.points.clone())
            }
            _ => None,
        })
        .collect()
}

fn click(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

// ----------------------------------------------------------------------------

/// Returns the anchor button rect and the popup rect.
fn popup_below_wide_button(right_to_left: bool) -> (Rect, Rect) {
    let ctx = context(right_to_left);
    let button = Cell::new(Rect::NOTHING);
    let popup = Cell::new(Rect::NOTHING);
    run_frames(&ctx, |ui| {
        ui.add_space(100.0);
        ui.horizontal(|ui| {
            ui.add_space(200.0);
            let response = ui.add_sized([300.0, 20.0], Button::new("Button"));
            button.set(response.rect);
            if let Some(inner) = Popup::from_response(&response)
                .open(true)
                .show(|ui| ui.label("Popup"))
            {
                popup.set(inner.response.rect);
            }
        });
    });
    assert!(popup.get().width() < button.get().width() - 50.0);
    (button.get(), popup.get())
}

#[test]
fn popup_aligned_to_the_left() {
    let (button, popup) = popup_below_wide_button(false);
    assert_eq!(popup.left(), button.left(), "{button:?} {popup:?}");
    assert!(popup.top() >= button.bottom());
}

#[test]
fn popup_aligned_to_the_right_right_to_left() {
    let (button, popup) = popup_below_wide_button(true);
    // Areas round their left-top corner to whole pixels, so with a fractional popup width
    // (real fonts) the right edge can be up to half a pixel off.
    // Painting rounds the frame edges to pixels too, so it still looks aligned.
    let ppp = Context::default().pixels_per_point();
    assert!(
        (popup.right() - button.right()).abs() <= 0.5 / ppp,
        "{button:?} {popup:?}"
    );
    assert!(popup.top() >= button.bottom());
}

// ----------------------------------------------------------------------------

/// Returns the x of the triangle center, the header text rect, and the header rect.
fn collapsing_header(right_to_left: bool) -> (f32, Rect, Rect) {
    let ctx = context(right_to_left);
    let header = Cell::new(Rect::NOTHING);
    let shapes = run_frames(&ctx, |ui| {
        let response = CollapsingHeader::new("Header").show(ui, |ui| ui.label("Body"));
        header.set(response.header_response.rect);
    });
    let triangles = triangles(&shapes, header.get());
    assert_eq!(triangles.len(), 1, "{triangles:?}");
    let icon_x = triangles[0].iter().map(|p| p.x).sum::<f32>() / 3.0;
    (icon_x, text_rect(&shapes, "Header"), header.get())
}

#[test]
fn collapsing_header_icon_before_text() {
    let (icon_x, text, header) = collapsing_header(false);
    assert!(icon_x < text.left(), "{icon_x} {text:?}");
    assert!(icon_x < header.center().x);
}

#[test]
fn collapsing_header_icon_before_text_right_to_left() {
    let (icon_x, text, header) = collapsing_header(true);
    assert!(icon_x > text.right(), "{icon_x} {text:?}");
    assert!(icon_x > header.center().x);
}

/// Returns the toggle button rect and the header label rect of [`CollapsingState::show_header`].
fn collapsing_state_header(right_to_left: bool) -> (Rect, Rect) {
    let ctx = context(right_to_left);
    let rects = Cell::new((Rect::NOTHING, Rect::NOTHING));
    run_frames(&ctx, |ui| {
        let (toggle, header, _) =
            CollapsingState::load_with_default_open(ui.ctx(), Id::unique("state"), false)
                .show_header(ui, |ui| ui.label("Header"))
                .body(|ui| ui.label("Body"));
        rects.set((toggle.rect, header.inner.rect));
    });
    rects.get()
}

#[test]
fn collapsing_state_toggle_before_header() {
    let (toggle, header) = collapsing_state_header(false);
    assert!(toggle.right() <= header.left(), "{toggle:?} {header:?}");
}

#[test]
fn collapsing_state_toggle_before_header_right_to_left() {
    let (toggle, header) = collapsing_state_header(true);
    assert!(toggle.left() >= header.right(), "{toggle:?} {header:?}");
}

/// Returns the available rect and the indented child's max rect.
fn indent(right_to_left: bool) -> (Rect, Rect) {
    let ctx = context(right_to_left);
    let rects = Cell::new((Rect::NOTHING, Rect::NOTHING));
    run(&ctx, vec![], |ui| {
        let available = ui.available_rect_before_wrap();
        let indented = ui.indent("indent", |ui| ui.max_rect()).inner;
        rects.set((available, indented));
    });
    rects.get()
}

#[test]
fn indent_from_the_left() {
    let (available, indented) = indent(false);
    let indent = Context::default().global_style().spacing.indent;
    assert_eq!(indented.left(), available.left() + indent);
    assert_eq!(indented.right(), available.right());
}

#[test]
fn indent_from_the_right_right_to_left() {
    let (available, indented) = indent(true);
    let indent = Context::default().global_style().spacing.indent;
    assert_eq!(indented.left(), available.left());
    assert_eq!(indented.right(), available.right() - indent);
}

// ----------------------------------------------------------------------------

/// Returns the rects of a closed combo box's arrow, selected text and label.
fn combo_box(right_to_left: bool) -> (Rect, Rect, Rect) {
    let ctx = context(right_to_left);
    let shapes = run_frames(&ctx, |ui| {
        ComboBox::from_label("Label")
            .selected_text("Selected")
            .show_ui(ui, |ui| ui.label("Option"));
    });
    let arrows = triangles(&shapes, Rect::EVERYTHING);
    assert_eq!(arrows.len(), 1, "{arrows:?}");
    (
        Rect::from_points(&arrows[0]),
        text_rect(&shapes, "Selected"),
        text_rect(&shapes, "Label"),
    )
}

#[test]
fn combo_box_arrow_at_the_right_label_after() {
    let (arrow, selected, label) = combo_box(false);
    assert!(arrow.left() > selected.right(), "{arrow:?} {selected:?}");
    assert!(label.left() > arrow.right(), "{label:?} {arrow:?}");
}

#[test]
fn combo_box_arrow_at_the_left_label_after_right_to_left() {
    let (arrow, selected, label) = combo_box(true);
    assert!(arrow.right() < selected.left(), "{arrow:?} {selected:?}");
    assert!(label.right() < arrow.left(), "{label:?} {arrow:?}");
}

/// Returns the icon rect and the label rect of a checkbox and of a radio button.
fn check_and_radio(right_to_left: bool) -> [(Rect, Rect); 2] {
    let ctx = context(right_to_left);
    let responses = Cell::new((Rect::NOTHING, Rect::NOTHING));
    let shapes = run_frames(&ctx, |ui| {
        let mut checked = true;
        let checkbox = ui.add(Checkbox::new(&mut checked, "Check")).rect;
        let radio = ui.add(RadioButton::new(true, "Radio")).rect;
        responses.set((checkbox, radio));
    });
    let (checkbox, radio) = responses.get();
    [
        (
            non_text_rect(&shapes, checkbox),
            text_rect(&shapes, "Check"),
        ),
        (non_text_rect(&shapes, radio), text_rect(&shapes, "Radio")),
    ]
}

#[test]
fn checkbox_and_radio_icons_left_of_label() {
    for (icon, label) in check_and_radio(false) {
        assert!(icon.right() <= label.left(), "{icon:?} {label:?}");
    }
}

#[test]
fn checkbox_and_radio_icons_right_of_label_right_to_left() {
    for (icon, label) in check_and_radio(true) {
        assert!(icon.left() >= label.right(), "{icon:?} {label:?}");
    }
}

// ----------------------------------------------------------------------------

/// A resizable area: returns its grip rect, its rect, and its rect after dragging the grip
/// 50 points outwards (right, or left in right-to-left).
fn resize_grip(right_to_left: bool) -> (Rect, Rect, Rect) {
    let ctx = context(right_to_left);
    let id = Id::unique("resize");
    let content = Cell::new(Rect::NOTHING);
    let show = |ui: &mut Ui| {
        ui.add_space(100.0);
        Resize::default()
            .id(id)
            .default_size([200.0, 100.0])
            .show(ui, |ui| {
                ui.label("Content");
                content.set(ui.max_rect());
            });
    };
    run_frames(&ctx, show);
    let grip = ctx
        .read_response(id.with("__resize_corner"))
        .expect("No resize grip")
        .rect;
    let before = content.get();

    let from = grip.center();
    let to = from + vec2(if right_to_left { -50.0 } else { 50.0 }, 0.0);
    run(
        &ctx,
        vec![Event::PointerMoved(from), click(from, true)],
        show,
    );
    run(&ctx, vec![Event::PointerMoved(from.lerp(to, 0.5))], show);
    run(&ctx, vec![Event::PointerMoved(to)], show);
    run(&ctx, vec![click(to, false)], show);
    run_frames(&ctx, show);
    (grip, before, content.get())
}

#[test]
fn resize_grip_at_the_bottom_right() {
    let (grip, before, after) = resize_grip(false);
    assert_eq!(
        grip.right_bottom(),
        before.right_bottom(),
        "{grip:?} {before:?}"
    );
    assert_eq!(after.left(), before.left());
    assert!(
        after.width() > before.width() + 40.0,
        "{before:?} {after:?}"
    );
}

#[test]
fn resize_grip_at_the_bottom_left_right_to_left() {
    let (grip, before, after) = resize_grip(true);
    assert_eq!(
        grip.left_bottom(),
        before.left_bottom(),
        "{grip:?} {before:?}"
    );
    assert_eq!(after.right(), before.right());
    assert!(
        after.width() > before.width() + 40.0,
        "{before:?} {after:?}"
    );
}

/// Returns how many line segments a resizable window paints near its bottom-left and
/// bottom-right corners.
fn window_grip_lines(right_to_left: bool) -> (usize, usize) {
    let ctx = context(right_to_left);
    let window = Cell::new(Rect::NOTHING);
    let shapes = run_frames(&ctx, |ui| {
        let response = Window::new("Window")
            .default_pos([200.0, 200.0])
            .default_size([200.0, 200.0])
            .show(ui.ctx(), |ui| ui.label("Hello"));
        window.set(response.expect("window").response.rect);
    });
    let window = window.get();
    let corner = |pos: Pos2| Rect::from_center_size(pos, vec2(30.0, 30.0));
    let lines_in = |rect: Rect| {
        shapes
            .iter()
            .filter(|clipped| match &clipped.shape {
                Shape::LineSegment { points, .. } => points.iter().all(|p| rect.contains(*p)),
                _ => false,
            })
            .count()
    };
    (
        lines_in(corner(window.left_bottom())),
        lines_in(corner(window.right_bottom())),
    )
}

#[test]
fn window_grip_at_the_bottom_right() {
    let (left, right) = window_grip_lines(false);
    assert!(left == 0 && right > 0, "{left} {right}");
}

#[test]
fn window_grip_at_the_bottom_left_right_to_left() {
    let (left, right) = window_grip_lines(true);
    assert!(left > 0 && right == 0, "{left} {right}");
}
