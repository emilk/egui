//! Regression tests for the menu item row surface
//! ([`egui::containers::menu::menu_item_button`]): a fixed-width checkmark
//! column at the reading start keeps check and no-check rows aligned, mirrors
//! with the interface direction, and composes with shortcuts and submenus.
//!
//! Every mirrored case also asserts its LTR counterpart, so this file doubles as
//! the LTR-unchanged regression. No snapshot assertions; painted text ink rects
//! and widget rects only.

use egui::epaint::Shape;
use egui::{
    Context, FullOutput, Id, Popup, Pos2, RawInput, Rect, Sense, Ui, Vec2,
    containers::menu::SubMenu, containers::menu::menu_item_button, pos2, vec2,
};

const SCREEN: Vec2 = vec2(800.0, 600.0);

fn input(size: Vec2, time: f64, events: Vec<egui::Event>) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        time: Some(time),
        events,
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, input: RawInput, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(input, run_ui);
    output.textures_delta.clear();
    output
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

/// The absolute ink rects of every painted text equal to `text`, in paint order.
fn painted_rects<'a>(texts: &'a [(String, Rect)], text: &str) -> Vec<&'a Rect> {
    texts
        .iter()
        .filter(|(painted, _)| painted == text)
        .map(|(_, rect)| rect)
        .collect()
}

/// Show a [`Popup::menu`] (forced open) with `body`, over the frames a popup
/// needs to reach its show pass, and return the last pass's output.
fn popup_output(rtl: bool, body: &mut dyn FnMut(&mut Ui)) -> FullOutput {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }

    let anchor_id = Id::unique("rtl-menu-item-anchor");
    let mut show = |ui: &mut Ui| {
        let response = ui.allocate_rect(
            Rect::from_min_size(pos2(300.0, 40.0), vec2(40.0, 20.0)),
            Sense::click(),
        );
        Popup::menu(&response)
            .anchor_widget(anchor_id)
            .open(true)
            .show(|ui| body(ui));
    };
    run(&ctx, input(SCREEN, 0.0, vec![]), &mut show); // invisible sizing pass
    run(&ctx, input(SCREEN, 0.1, vec![]), &mut show) // show pass
}

/// The area rect of a submenu opened by hovering its row, plus the last pass's
/// output. Frames: parent sizing pass, parent show pass, hover the row (opens
/// the submenu as its sizing pass), settle (the submenu's show pass).
fn hovered_submenu_row(
    rtl: bool,
    row: &mut dyn FnMut(&mut Ui) -> egui::Response,
) -> (FullOutput, Rect, Rect) {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }

    let anchor_id = Id::unique("rtl-menu-item-submenu-anchor");
    let row_id = core::cell::RefCell::new(Id::NULL);
    let row_rect = core::cell::RefCell::new(Rect::NOTHING);

    let mut show = |ui: &mut Ui| {
        let response = ui.allocate_rect(
            Rect::from_min_size(pos2(300.0, 40.0), vec2(40.0, 20.0)),
            Sense::click(),
        );
        Popup::menu(&response)
            .anchor_widget(anchor_id)
            .open(true)
            .show(|ui| {
                let row = row(ui);
                *row_id.borrow_mut() = row.id;
                *row_rect.borrow_mut() = row.rect;
            });
    };

    run(&ctx, input(SCREEN, 0.0, vec![]), &mut show); // parent sizing pass
    run(&ctx, input(SCREEN, 0.1, vec![]), &mut show); // parent show pass
    let center = row_rect.borrow().center();
    run(
        &ctx,
        input(SCREEN, 0.2, vec![egui::Event::PointerMoved(center)]),
        &mut show,
    ); // hover opens the submenu (its sizing pass)
    let output = run(&ctx, input(SCREEN, 0.3, vec![]), &mut show); // submenu show pass

    let popup_id = SubMenu::id_from_widget_id(row_id.borrow().to_owned());
    let popup = ctx
        .memory(|memory| memory.area_rect(popup_id))
        .expect("the submenu should have been shown");
    let row = row_rect.borrow().to_owned();
    (output, popup, row)
}

// ---------------------------------------------------------------------------

/// The checkmark column keeps check and no-check rows aligned at the reading
/// start under RTL: both labels share the same physical right edge, and the
/// checked row paints the mark right of its label.
///
/// A naive `format!("✔ {label}")` prefix would measure per-row and misalign the
/// rows instead of reserving one fixed column for all of them.
#[test]
fn gutter_aligns_rows_at_the_reading_start_under_rtl() {
    let labels = core::cell::RefCell::new(Vec::new());
    let output = popup_output(true, &mut |ui: &mut Ui| {
        let checked = ui.add(menu_item_button(ui, "خيار", true, None as Option<&str>));
        let unchecked = ui.add(menu_item_button(ui, "خيار", false, None as Option<&str>));
        *labels.borrow_mut() = vec![checked.rect, unchecked.rect];
    });

    let texts = painted_texts(&output);
    let option_labels = painted_rects(&texts, "خيار");
    assert_eq!(
        option_labels.len(),
        2,
        "both rows should paint their label, got {texts:?}"
    );
    assert!(
        (option_labels[0].right() - option_labels[1].right()).abs() < 2.0,
        "RTL: check and no-check labels should align at the reading start \
         (checked {:?}, unchecked {:?})",
        option_labels[0],
        option_labels[1]
    );

    let marks = painted_rects(&texts, "✔ ");
    assert_eq!(
        marks.len(),
        1,
        "the checked row should paint exactly one mark, got {texts:?}"
    );
    assert!(
        marks[0].is_positive(),
        "the mark should have ink (no missing-glyph tofu): {:?}",
        marks[0]
    );
    assert!(
        marks[0].left() > option_labels[0].right(),
        "RTL: the mark should sit right of its label (mark {:?}, label {:?})",
        marks[0],
        option_labels[0]
    );
}

/// LTR twin: an unchecked row stays a plain button (pixel-identical), a checked
/// row indents its label behind the mark at the physical left.
#[test]
fn ltr_rows_keep_the_plain_button_and_indent_checked_ones() {
    let rects = core::cell::RefCell::new(Vec::new());
    let output = popup_output(false, &mut |ui: &mut Ui| {
        let checked = ui.add(menu_item_button(ui, "خيار", true, None as Option<&str>));
        let unchecked = ui.add(menu_item_button(ui, "خيار", false, None as Option<&str>));
        let plain = ui.button("خيار");
        *rects.borrow_mut() = vec![checked.rect, unchecked.rect, plain.rect];
    });

    let texts = painted_texts(&output);
    let option_labels = painted_rects(&texts, "خيار");
    assert_eq!(
        option_labels.len(),
        3,
        "both rows plus the plain button should paint their label, got {texts:?}"
    );

    let marks = painted_rects(&texts, "✔ ");
    assert_eq!(
        marks.len(),
        1,
        "only the checked row should paint a mark, got {texts:?}"
    );
    assert!(
        marks[0].right() < option_labels[0].left(),
        "LTR: the mark should sit left of its label (mark {:?}, label {:?})",
        marks[0],
        option_labels[0]
    );
    // The unchecked row's label paints exactly where the plain button's label
    // paints (same x, same width; the rows sit at different y):
    let (row, plain) = (option_labels[1], option_labels[2]);
    assert!(
        row.min.x == plain.min.x && row.width() == plain.width(),
        "LTR: the unchecked row should be pixel-identical to a plain button \
         (row {row:?}, plain {plain:?})"
    );
    assert!(
        option_labels[0].left() > option_labels[1].left(),
        "LTR: the checked row's label should be indented behind the mark \
         (checked {:?}, unchecked {:?})",
        option_labels[0],
        option_labels[1]
    );
}

/// The optional shortcut lands at the reading end of the composed row: left of
/// the label under RTL, right of it under LTR.
#[test]
fn shortcut_sits_at_the_reading_end_in_both_directions() {
    for (rtl, want_shortcut_left) in [(true, true), (false, false)] {
        let output = popup_output(rtl, &mut |ui: &mut Ui| {
            ui.add(menu_item_button(ui, "Open", false, Some("Ctrl+O")));
        });

        let texts = painted_texts(&output);
        let open = painted_rects(&texts, "Open");
        let shortcut = painted_rects(&texts, "Ctrl+O");
        assert!(
            open.len() == 1 && shortcut.len() == 1,
            "the label and the shortcut should both paint (rtl = {rtl}), got {texts:?}"
        );
        assert_eq!(
            shortcut[0].left() < open[0].left(),
            want_shortcut_left,
            "the shortcut sits at the reading end (rtl = {rtl}: \
             'Open' at {:?}, 'Ctrl+O' at {:?})",
            open[0],
            shortcut[0]
        );
    }
}

/// The reserved column width is direction-invariant: the distance between a
/// checked row's reading-start edge and its label's ink edge is the same under
/// RTL and LTR (same font, same formula).
#[test]
fn gutter_width_is_direction_invariant() {
    let rects = core::cell::RefCell::new(Rect::NOTHING);
    let output = popup_output(true, &mut |ui: &mut Ui| {
        let row = ui.add(menu_item_button(ui, "Item", true, None as Option<&str>));
        *rects.borrow_mut() = row.rect;
    });
    let rtl_texts = painted_texts(&output);
    let rtl_label = painted_rects(&rtl_texts, "Item");
    assert_eq!(rtl_label.len(), 1, "the label should paint once");
    let rtl_offset = rects.borrow().max.x - rtl_label[0].max.x;

    let rects = core::cell::RefCell::new(Rect::NOTHING);
    let output = popup_output(false, &mut |ui: &mut Ui| {
        let row = ui.add(menu_item_button(ui, "Item", true, None as Option<&str>));
        *rects.borrow_mut() = row.rect;
    });
    let ltr_texts = painted_texts(&output);
    let ltr_label = painted_rects(&ltr_texts, "Item");
    assert_eq!(ltr_label.len(), 1, "the label should paint once");
    let ltr_offset = ltr_label[0].min.x - rects.borrow().min.x;

    assert!(
        (rtl_offset - ltr_offset).abs() < 2.0,
        "the gutter should reserve the same width in both directions \
         (RTL {rtl_offset:.1}, LTR {ltr_offset:.1})"
    );
}

/// A submenu row built through the same surface still opens on the
/// reading-direction side of its row, with the checkmark column present.
#[test]
fn submenu_row_through_the_item_surface_opens_on_the_reading_direction_side() {
    let row = &mut |ui: &mut Ui| {
        use egui::containers::menu::SubMenuButton;
        SubMenuButton::from_button(menu_item_button(ui, "خيار", true, None as Option<&str>))
            .ui(ui, |ui| {
                let _ = ui.button("Item in submenu");
            })
            .0
    };

    let (output, popup, row_rect) = hovered_submenu_row(true, row);

    let texts = painted_texts(&output);
    let marks = painted_rects(&texts, "✔ ");
    assert!(
        marks.len() == 1 && marks[0].is_positive(),
        "the submenu row should paint its mark, got {texts:?}"
    );
    assert!(
        popup.max.x <= row_rect.min.x + 2.0,
        "RTL: the submenu should open on the reading-end (left) side of its row \
         (popup {popup:?}, row {row_rect:?})"
    );
}

/// A caller-provided [`egui::LayoutJob`] label flows through the row surface.
#[test]
fn layout_job_label_flows_through_the_row_surface() {
    let mut job = egui::text::LayoutJob::simple(
        "خيار".to_owned(),
        egui::FontId::default(),
        egui::Color32::PLACEHOLDER,
        f32::INFINITY,
    );
    job.direction = Some(egui::epaint::text::TextDirection::RightToLeft);
    let output = popup_output(true, &mut |ui: &mut Ui| {
        ui.add(menu_item_button(
            ui,
            job.clone(),
            true,
            None as Option<&str>,
        ));
    });

    let texts = painted_texts(&output);
    let label = painted_rects(&texts, "خيار");
    assert_eq!(
        label.len(),
        1,
        "the layout-job label should paint, got {texts:?}"
    );
    assert!(
        label[0].is_positive(),
        "the layout-job label should have ink: {:?}",
        label[0]
    );
}
