//! A pointer move still runs the interface pass. The redraw is skipped only
//! when that pass paints the same picture as the last tessellation.

use std::sync::Arc;

use egui::{
    Color32, ColorImage, Context, CursorIcon, Event, Id, Key, Modifiers, PaintCallback, Pos2,
    RawInput, Rect, Sense, Shape, TextureOptions, Ui, Vec2, ViewportId, pos2, vec2,
};

struct Frame {
    skip: bool,
    cursor: CursorIcon,
    shapes: Vec<egui::epaint::ClippedShape>,
    delta_empty: bool,
}

fn raw(events: Vec<Event>, screen: Vec2, pixels_per_point: f32) -> RawInput {
    let mut input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, screen)),
        events,
        ..Default::default()
    };
    input
        .viewports
        .entry(ViewportId::ROOT)
        .or_default()
        .native_pixels_per_point = Some(pixels_per_point);
    input
}

fn moved(pos: Pos2) -> RawInput {
    raw(vec![Event::PointerMoved(pos)], vec2(200.0, 200.0), 1.0)
}

fn show(ctx: &Context, input: RawInput, add: impl FnMut(&mut Ui)) -> Frame {
    let mut output = ctx.run_ui(input, add);
    let frame = Frame {
        skip: output.skip_redraw,
        cursor: output.platform_output.cursor_icon,
        shapes: output.shapes.clone(),
        delta_empty: output.textures_delta.is_empty(),
    };
    // The integration tessellates only when it is going to paint. That call
    // records the picture the next frame compares against.
    if !output.skip_redraw {
        let _primitives =
            ctx.tessellate(core::mem::take(&mut output.shapes), output.pixels_per_point);
    }
    output.drop_without_applying_deltas();
    frame
}

fn paint_hover(ui: &mut Ui) {
    let rect = Rect::from_min_size(pos2(40.0, 40.0), vec2(30.0, 20.0));
    let response = ui.interact(rect, Id::unique("hit"), Sense::hover());
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
    }
    let color = if response.hovered() {
        Color32::RED
    } else {
        Color32::BLUE
    };
    ui.painter().rect_filled(rect, 0.0, color);
}

fn until_skip(ctx: &Context, pos: Pos2) -> Frame {
    for _ in 0..12 {
        let frame = show(ctx, moved(pos), paint_hover);
        if frame.skip {
            return frame;
        }
    }
    panic!("a pointer move over an unchanged picture still asked for a redraw");
}

#[test]
fn pointer_move_skips_redraw_until_the_hover_picture_changes() {
    let ctx = Context::default();
    let outside = until_skip(&ctx, pos2(1.0, 1.0));

    let still_outside = show(&ctx, moved(pos2(10.0, 10.0)), paint_hover);
    assert!(still_outside.skip);
    assert_eq!(still_outside.shapes, outside.shapes);
    assert_eq!(still_outside.cursor, CursorIcon::Default);

    let onto = show(&ctx, moved(pos2(50.0, 50.0)), paint_hover);
    assert!(
        !onto.skip,
        "crossing into the widget changes the hover color"
    );
    assert_ne!(onto.shapes, outside.shapes);
    assert_eq!(onto.cursor, CursorIcon::Crosshair);

    let inside = show(&ctx, moved(pos2(55.0, 48.0)), paint_hover);
    assert!(
        inside.skip,
        "a slide inside the same widget rebuilds the same shapes"
    );
    assert_eq!(inside.shapes, onto.shapes);
    assert_eq!(inside.cursor, CursorIcon::Crosshair);

    let off = show(&ctx, moved(pos2(1.0, 1.0)), paint_hover);
    assert!(!off.skip, "leaving the widget clears the hover color");
    assert_eq!(off.shapes, outside.shapes);
}

#[test]
fn a_slide_that_paints_the_pointer_does_not_skip() {
    let ctx = Context::default();
    let paint = |ui: &mut Ui| {
        let pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or_default();
        ui.painter().rect_filled(
            Rect::from_center_size(pos, vec2(4.0, 4.0)),
            0.0,
            Color32::YELLOW,
        );
    };

    let mut previous: Option<Vec<egui::epaint::ClippedShape>> = None;
    for step in 0..12 {
        let frame = show(&ctx, moved(pos2(20.0 + step as f32, 20.0)), paint);
        if frame.delta_empty
            && previous
                .as_ref()
                .is_some_and(|previous| frame.shapes != *previous)
        {
            assert!(
                !frame.skip,
                "the pointer moved inside one widget and the painted rect moved with it"
            );
            return;
        }
        if frame.delta_empty {
            previous = Some(frame.shapes);
        }
    }
    panic!("the font atlas never settled, so the pointer-follow case did not run");
}

#[test]
fn an_application_repaint_paints_even_when_the_shapes_match() {
    let ctx = Context::default();
    let settled = until_skip(&ctx, pos2(1.0, 1.0));

    let asked = show(&ctx, moved(pos2(8.0, 8.0)), |ui| {
        paint_hover(ui);
        ui.ctx().request_repaint();
    });
    assert!(!asked.skip);
    assert!(asked.delta_empty);
    assert_eq!(asked.shapes, settled.shapes);

    let woken = show(&ctx, moved(pos2(12.0, 4.0)), paint_hover);
    assert!(
        !woken.skip,
        "the frame the application asked for still paints"
    );
    assert_eq!(woken.shapes, settled.shapes);

    let after = show(&ctx, moved(pos2(3.0, 6.0)), paint_hover);
    assert!(after.skip);
    assert_eq!(after.shapes, settled.shapes);
}

#[test]
fn a_callback_paints_even_when_the_shapes_compare_equal() {
    let ctx = Context::default();
    let callback = Arc::new(7u8);
    let paint = |ui: &mut Ui| {
        ui.painter().rect_filled(
            Rect::from_min_size(pos2(0.0, 0.0), vec2(4.0, 4.0)),
            0.0,
            Color32::WHITE,
        );
        // Nested, so a check of only the top-level enum misses it.
        // The same callback allocation compares equal. The scene inside it
        // does not.
        ui.painter()
            .add(Shape::Vec(vec![Shape::Callback(PaintCallback {
                rect: Rect::from_min_size(pos2(0.0, 0.0), vec2(10.0, 10.0)),
                callback: Arc::<u8>::clone(&callback),
            })]));
    };

    let mut previous: Option<Vec<egui::epaint::ClippedShape>> = None;
    for step in 0..12 {
        let frame = show(&ctx, moved(pos2(5.0 + step as f32, 5.0)), &paint);
        if frame.delta_empty && previous.as_ref() == Some(&frame.shapes) {
            assert!(
                !frame.skip,
                "a callback compares equal while the scene inside it can move"
            );
            return;
        }
        if frame.delta_empty {
            previous = Some(frame.shapes);
        }
    }
    panic!("two callback frames never produced the same shape list");
}

#[test]
fn a_texture_delta_paints_even_when_the_shapes_match() {
    let ctx = Context::default();
    let settled = until_skip(&ctx, pos2(1.0, 1.0));
    let mut handle = None;
    let updated = show(&ctx, moved(pos2(6.0, 2.0)), |ui| {
        paint_hover(ui);
        handle = Some(ui.ctx().load_texture(
            "video",
            ColorImage::filled([2, 2], Color32::GREEN),
            TextureOptions::default(),
        ));
    });
    assert!(!updated.delta_empty);
    assert_eq!(updated.shapes, settled.shapes);
    assert!(!updated.skip);
    drop(handle);
}

#[test]
fn a_frame_that_is_not_a_pointer_move_paints() {
    let ctx = Context::default();
    let settled = until_skip(&ctx, pos2(1.0, 1.0));

    let keyed = show(
        &ctx,
        raw(
            vec![Event::Key {
                key: Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::default(),
            }],
            vec2(200.0, 200.0),
            1.0,
        ),
        paint_hover,
    );
    assert_eq!(keyed.shapes, settled.shapes);
    assert!(!keyed.skip);

    let idle = show(&ctx, raw(vec![], vec2(200.0, 200.0), 1.0), paint_hover);
    assert_eq!(idle.shapes, settled.shapes);
    assert!(!idle.skip);

    let scaled = show(
        &ctx,
        raw(
            vec![Event::PointerMoved(pos2(4.0, 4.0))],
            vec2(200.0, 200.0),
            2.0,
        ),
        paint_hover,
    );
    assert_eq!(scaled.shapes, settled.shapes);
    assert!(scaled.delta_empty);
    assert!(!scaled.skip);
}

#[test]
fn a_resize_with_the_same_shapes_still_paints() {
    let ctx = Context::default();
    let paint = |ui: &mut Ui| {
        // A clip that sits inside both window sizes, so the shape list does
        // not grow with the window. The new pixels are still unpainted.
        let clip = Rect::from_min_size(pos2(10.0, 10.0), vec2(20.0, 20.0));
        ui.painter()
            .with_clip_rect(clip)
            .rect_filled(clip, 0.0, Color32::BLUE);
    };
    let input = |screen: Vec2| raw(vec![Event::PointerMoved(pos2(1.0, 1.0))], screen, 1.0);

    let mut settled = None;
    for _ in 0..12 {
        let frame = show(&ctx, input(vec2(200.0, 200.0)), paint);
        if frame.skip {
            settled = Some(frame);
            break;
        }
    }
    let settled = settled.expect("a pointer move over a fixed clip never skipped the redraw");

    let resized = show(&ctx, input(vec2(240.0, 200.0)), paint);
    assert_eq!(resized.shapes, settled.shapes);
    assert!(!resized.skip);
}
