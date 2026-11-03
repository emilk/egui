//! Regression tests for atom-based widgets following the layout's reading direction:
//! `ContainerAtom::measure` defaults the atom direction from the same predicate
//! `Placer::prefer_right_to_left` uses to mirror nested rows, and
//! [`egui::Button::direction`] overrides it.
//!
//! Every mirrored case also asserts its LTR counterpart, so this file doubles as the
//! LTR-unchanged regression. No snapshot assertions; painted text edges and icon
//! geometry only.

use egui::epaint::Shape;
use egui::{
    Align, Button, Checkbox, Context, Direction, FullOutput, Layout, Pos2, RadioButton, RawInput,
    Rect, RichText, Ui, Vec2, Window, vec2,
};

const SCREEN: Vec2 = vec2(400.0, 300.0);

fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, input: RawInput, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(input, run_ui);
    output.textures_delta.clear();
    output
}

/// The horizontal row layout matching the reading direction: right-to-left under RTL,
/// left-to-right under LTR.
fn row_layout(rtl: bool) -> Layout {
    if rtl {
        Layout::right_to_left(Align::Min)
    } else {
        Layout::left_to_right(Align::Min)
    }
}

/// (text, absolute ink rect) for every text shape painted this pass.
///
/// `Galley::mesh_bounds` excludes leading/trailing whitespace, so the rect is the true
/// ink bounds of the glyphs, translated to their absolute position.
fn painted_texts(output: &FullOutput) -> Vec<(String, Rect)> {
    fn collect(shape: &Shape, out: &mut Vec<(String, Rect)>) {
        match shape {
            Shape::Text(text) => {
                out.push((
                    text.galley.job.text.clone(),
                    text.galley.mesh_bounds.translate(text.pos.to_vec2()),
                ));
            }
            Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut out);
    }
    out
}

/// The absolute ink rect of the first painted text equal to `text`, or `None`.
fn painted_rect<'a>(texts: &'a [(String, Rect)], text: &str) -> Option<&'a Rect> {
    texts
        .iter()
        .find(|(painted, _)| painted == text)
        .map(|(_, rect)| rect)
}

/// The painted line segments of this pass, as `[start, end]` point pairs.
fn line_segments(output: &FullOutput) -> Vec<[Pos2; 2]> {
    fn collect(shape: &Shape, out: &mut Vec<[Pos2; 2]>) {
        match shape {
            Shape::LineSegment { points, .. } => out.push(*points),
            Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut out);
    }
    out
}

// ---------------------------------------------------------------------------

/// A2: a `Button::new("Open").shortcut_text("Ctrl+O")` inside a right-to-left layout
/// paints the shortcut LEFT of the label; in the default layout it stays RIGHT of the
/// label. LTR pixels are unchanged.
#[test]
fn button_atoms_follow_the_layout_direction() {
    for (rtl, want_shortcut_left) in [(false, false), (true, true)] {
        let ctx = Context::default();
        let output = run(&ctx, input(SCREEN), |ui| {
            if rtl {
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    ui.add(Button::new("Open").shortcut_text("Ctrl+O"));
                });
            } else {
                ui.add(Button::new("Open").shortcut_text("Ctrl+O"));
            }
        });

        let texts = painted_texts(&output);
        let open = painted_rect(&texts, "Open");
        let shortcut = painted_rect(&texts, "Ctrl+O");
        assert!(
            open.is_some() && shortcut.is_some(),
            "the label and the shortcut should both paint (rtl = {rtl}), got {texts:?}"
        );
        let (open, shortcut) = (
            open.map_or(f32::INFINITY, |rect| rect.left()),
            shortcut.map_or(f32::NEG_INFINITY, |rect| rect.left()),
        );
        assert_eq!(
            shortcut < open,
            want_shortcut_left,
            "the shortcut sits at the reading end (rtl = {rtl}: \
             'Open' at {open}, 'Ctrl+O' at {shortcut})"
        );
    }
}

/// A2 (flag case): with [`egui::Context::set_right_to_left`], the atoms of a button in a
/// `ui.horizontal` row mirror the way the row itself mirrors; with the flag off the
/// shortcut stays right of the label.
#[test]
fn button_shortcut_follows_the_interface_flag() {
    for (rtl, want_shortcut_left) in [(false, false), (true, true)] {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let output = run(&ctx, input(SCREEN), |ui| {
            ui.horizontal(|ui| {
                ui.add(Button::new("Open").shortcut_text("Ctrl+O"));
            });
        });

        let texts = painted_texts(&output);
        let open = painted_rect(&texts, "Open");
        let shortcut = painted_rect(&texts, "Ctrl+O");
        assert!(
            open.is_some() && shortcut.is_some(),
            "the label and the shortcut should both paint (rtl = {rtl}), got {texts:?}"
        );
        let (open, shortcut) = (
            open.map_or(f32::INFINITY, |rect| rect.left()),
            shortcut.map_or(f32::NEG_INFINITY, |rect| rect.left()),
        );
        assert_eq!(
            shortcut < open,
            want_shortcut_left,
            "the shortcut mirrors with the row (rtl = {rtl}: \
             'Open' at {open}, 'Ctrl+O' at {shortcut})"
        );
    }
}

/// A4: `left_text`/`right_text` extras keep their logical reading order and swap
/// physical sides under RTL (`right_text` lands at the physical left, `left_text` at
/// the physical right); the LTR counterparts mirror them.
#[test]
fn left_text_and_right_text_mirror_in_rtl() {
    for (rtl, want_right_text_left) in [(false, false), (true, true)] {
        let ctx = Context::default();
        let output = run(&ctx, input(SCREEN), |ui| {
            ui.with_layout(row_layout(rtl), |ui| {
                ui.add(Button::new("Open").right_text(RichText::new("Ctrl+O").weak()));
            });
        });

        let texts = painted_texts(&output);
        let open = painted_rect(&texts, "Open");
        let extra = painted_rect(&texts, "Ctrl+O");
        assert!(
            open.is_some() && extra.is_some(),
            "the label and the extra should both paint (rtl = {rtl}), got {texts:?}"
        );
        let (open, extra) = (
            open.map_or(f32::INFINITY, |rect| rect.left()),
            extra.map_or(f32::NEG_INFINITY, |rect| rect.left()),
        );
        assert_eq!(
            extra < open,
            want_right_text_left,
            "right_text lands at the reading end (rtl = {rtl}: \
             'Open' at {open}, 'Ctrl+O' at {extra})"
        );
    }

    for (rtl, want_left_text_left) in [(false, true), (true, false)] {
        let ctx = Context::default();
        let output = run(&ctx, input(SCREEN), |ui| {
            ui.with_layout(row_layout(rtl), |ui| {
                ui.add(Button::new("Open").left_text(RichText::new("Ctrl+O").weak()));
            });
        });

        let texts = painted_texts(&output);
        let open = painted_rect(&texts, "Open");
        let extra = painted_rect(&texts, "Ctrl+O");
        assert!(
            open.is_some() && extra.is_some(),
            "the label and the extra should both paint (rtl = {rtl}), got {texts:?}"
        );
        let (open, extra) = (
            open.map_or(f32::INFINITY, |rect| rect.left()),
            extra.map_or(f32::NEG_INFINITY, |rect| rect.left()),
        );
        assert_eq!(
            extra < open,
            want_left_text_left,
            "left_text lands at the reading start (rtl = {rtl}: \
             'Open' at {open}, 'Ctrl+O' at {extra})"
        );
    }
}

/// A3: an explicit `Button::direction` wins over the layout default, in both directions.
#[test]
fn explicit_direction_overrides_the_layout() {
    // Forced LTR inside a right-to-left layout keeps today's physical order
    // (opt-out for media-transport-style widgets):
    let ctx = Context::default();
    let output = run(&ctx, input(SCREEN), |ui| {
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            ui.add(
                Button::new("Open")
                    .direction(Direction::LeftToRight)
                    .shortcut_text("Ctrl+O"),
            );
        });
    });

    let texts = painted_texts(&output);
    let open = painted_rect(&texts, "Open");
    let shortcut = painted_rect(&texts, "Ctrl+O");
    assert!(
        open.is_some() && shortcut.is_some(),
        "the label and the shortcut should both paint, got {texts:?}"
    );
    assert!(
        open.map_or(f32::INFINITY, |rect| rect.left())
            < shortcut.map_or(f32::NEG_INFINITY, |rect| rect.left()),
        "the forced-LTR button keeps the label left of the shortcut, got {texts:?}"
    );

    // ...and the inverse override mirrors inside the default layout:
    let ctx = Context::default();
    let output = run(&ctx, input(SCREEN), |ui| {
        ui.add(
            Button::new("Open")
                .direction(Direction::RightToLeft)
                .shortcut_text("Ctrl+O"),
        );
    });

    let texts = painted_texts(&output);
    let open = painted_rect(&texts, "Open");
    let shortcut = painted_rect(&texts, "Ctrl+O");
    assert!(
        open.is_some() && shortcut.is_some(),
        "the label and the shortcut should both paint, got {texts:?}"
    );
    assert!(
        shortcut.map_or(f32::NEG_INFINITY, |rect| rect.left())
            < open.map_or(f32::INFINITY, |rect| rect.left()),
        "the forced-RTL button puts the shortcut left of the label, got {texts:?}"
    );
}

/// A5: a button inside a vertical `Align::Max` body (which
/// `Layout::prefer_right_to_left` reports as right-to-left, and inside which
/// `ui.horizontal` already mirrors) mirrors its atoms too; a vertical `Align::Min`
/// body stays left-to-right with the flag off.
#[test]
fn vertical_max_body_mirrors_atoms() {
    for (cross_align, want_shortcut_left) in [(Align::Max, true), (Align::Min, false)] {
        let ctx = Context::default();
        let output = run(&ctx, input(SCREEN), |ui| {
            ui.with_layout(Layout::top_down_justified(cross_align), |ui| {
                ui.add(Button::new("Open").shortcut_text("Ctrl+O"));
            });
        });

        let texts = painted_texts(&output);
        let open = painted_rect(&texts, "Open");
        let shortcut = painted_rect(&texts, "Ctrl+O");
        assert!(
            open.is_some() && shortcut.is_some(),
            "the label and the shortcut should both paint (cross_align = {cross_align:?}), \
             got {texts:?}"
        );
        let (open, shortcut) = (
            open.map_or(f32::INFINITY, |rect| rect.left()),
            shortcut.map_or(f32::NEG_INFINITY, |rect| rect.left()),
        );
        assert_eq!(
            shortcut < open,
            want_shortcut_left,
            "atoms read in the body's reading direction (cross_align = {cross_align:?}: \
             'Open' at {open}, 'Ctrl+O' at {shortcut})"
        );
    }
}

/// A6: a checkbox/radio inside an explicit right-to-left row has its icon on the
/// reading-start (right) side with the flag off AND with the flag on (the container
/// mirrors exactly once); a left-to-right row keeps the icon on the left in both flag
/// states.
#[test]
fn checkbox_and_radio_icons_stay_reading_start_in_rtl_rows() {
    for (rtl, want_icon_right) in [(false, false), (true, true)] {
        for flag in [false, true] {
            let ctx = Context::default();
            if flag {
                ctx.set_right_to_left(true);
            }
            let mut checked = false;
            let mut widget = Rect::NOTHING;
            let output = run(&ctx, input(vec2(200.0, 100.0)), |ui| {
                ui.with_layout(row_layout(rtl), |ui| {
                    let response = ui.add(Checkbox::new(&mut checked, ""));
                    widget = response.rect;
                });
            });

            let icons: Vec<Rect> = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    Shape::Rect(rect_shape) => Some(rect_shape.rect),
                    _ => None,
                })
                .filter(|rect| {
                    rect.is_positive()
                        && widget.contains_rect(*rect)
                        && rect.width() < widget.width() - 1.0
                })
                .collect();
            assert_eq!(
                icons.len(),
                1,
                "expected exactly one painted checkbox box (rtl = {rtl}, flag = {flag}), \
                 got {icons:?}"
            );
            assert_eq!(
                icons[0].center().x > widget.center().x,
                want_icon_right,
                "the checkbox icon sits on the reading start (rtl = {rtl}, flag = {flag}: \
                 icon {:?}, widget {widget:?})",
                icons[0]
            );
        }
    }

    for (rtl, want_icon_right) in [(false, false), (true, true)] {
        for flag in [false, true] {
            let ctx = Context::default();
            if flag {
                ctx.set_right_to_left(true);
            }
            let mut widget = Rect::NOTHING;
            let output = run(&ctx, input(vec2(200.0, 100.0)), |ui| {
                ui.with_layout(row_layout(rtl), |ui| {
                    let response = ui.add(RadioButton::new(false, ""));
                    widget = response.rect;
                });
            });

            let icons: Vec<Pos2> = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    Shape::Circle(circle) => Some(circle.center),
                    _ => None,
                })
                .filter(|center| widget.contains(*center))
                .collect();
            assert_eq!(
                icons.len(),
                1,
                "expected exactly one painted radio circle (rtl = {rtl}, flag = {flag}), \
                 got {icons:?}"
            );
            assert_eq!(
                icons[0].x > widget.center().x,
                want_icon_right,
                "the radio icon sits on the reading start (rtl = {rtl}, flag = {flag}: \
                 icon at {}, widget {widget:?})",
                icons[0].x
            );
        }
    }
}

/// A7: with the interface flag on, the window close button stays at the top-left and
/// the title-bar carve stays live: the title bar mirrors its atom list itself, so the
/// container must not mirror it a second time. The LTR counterpart keeps the close
/// button at the top-right.
#[test]
fn window_title_bar_atoms_do_not_double_mirror() {
    for (rtl, want_close_left) in [(false, false), (true, true)] {
        let mut open = true;
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }

        // Settle over a few passes (the flag is set before the first one); the window
        // needs `.open` to own a close button at all:
        let show = |ctx: &Context, open: &mut bool| {
            let mut output = run(ctx, input(SCREEN), |ui| {
                window(open).show(ui.ctx(), |ui| ui.label("body"));
            });
            for _ in 0..2 {
                output = run(ctx, input(SCREEN), |ui| {
                    window(open).show(ui.ctx(), |ui| ui.label("body"));
                });
            }
            output
        };
        let output = show(&ctx, &mut open);

        // The close button paints the only line segments of a window with no resize
        // corner (`.resizable(false)`) and no collapse icon (`.collapsible(false)`):
        let segments = line_segments(&output);
        assert_eq!(
            segments.len(),
            2,
            "expected exactly the two close-button strokes (rtl = {rtl}), got {segments:?}"
        );
        let (min_x, max_x) =
            segments
                .iter()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), points| {
                    (
                        min.min(points[0].x).min(points[1].x),
                        max.max(points[0].x).max(points[1].x),
                    )
                });
        let close_center_x = (min_x + max_x) * 0.5;

        let texts = painted_texts(&output);
        let title = painted_rect(&texts, "Demo");
        assert!(
            title.is_some(),
            "the window title should paint (rtl = {rtl}), got {texts:?}"
        );
        let title = title.map_or(Rect::NOTHING, |rect| *rect);

        assert_eq!(
            close_center_x < title.left(),
            want_close_left,
            "the close button sits at the reading start (rtl = {rtl}: close centre {close_center_x}, \
             title {title:?})"
        );
    }
}

fn window(open: &mut bool) -> Window<'_> {
    Window::new("Demo")
        .open(open)
        .collapsible(false)
        .resizable(false)
}
