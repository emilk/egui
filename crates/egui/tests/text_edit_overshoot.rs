use egui::{
    Color32, Context, FontId, RawInput, Rect, Shape, TextBuffer, TextEdit, Ui, pos2,
    text::LayoutJob, vec2,
};

/// Glyphs taller than their row, like Thai marks or stacked Vietnamese accents
/// in a font whose line height doesn't cover them, must not be clipped.
#[test]
fn text_edit_does_not_clip_glyphs_that_overshoot_the_row() {
    let ctx = Context::default();
    let mut text = "Ẫ".repeat(40);
    let mut field = Rect::NOTHING;
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0))),
            ..Default::default()
        },
        |ui| {
            let mut layouter = |ui: &Ui, text: &dyn TextBuffer, _wrap_width: f32| {
                let mut job = LayoutJob::simple_singleline(
                    text.as_str().to_owned(),
                    FontId::proportional(20.0),
                    Color32::WHITE,
                );
                job.sections[0].format.line_height = Some(8.0);
                ui.fonts_mut(|f| f.layout_job(job))
            };
            ui.add_space(50.0);
            field = TextEdit::singleline(&mut text)
                .desired_width(100.0)
                .layouter(&mut layouter)
                .show(ui)
                .response
                .rect;
        },
    );
    output.textures_delta.clear();

    let text_shape = output
        .shapes
        .iter()
        .find(|shape| matches!(shape.shape, Shape::Text(_)))
        .expect("the text is painted");
    let glyphs = text_shape.shape.visual_bounding_rect();
    let clip = text_shape.clip_rect;

    assert!(
        glyphs.top() < field.top() || field.bottom() < glyphs.bottom(),
        "the test glyphs overshoot the field: {glyphs:?} vs {field:?}"
    );
    assert!(
        clip.top() <= glyphs.top() && glyphs.bottom() <= clip.bottom(),
        "glyphs {glyphs:?} are clipped vertically by {clip:?}"
    );
    assert!(
        field.left() <= clip.left() && clip.right() <= field.right(),
        "text that scrolls is still clipped to the field: {clip:?} vs {field:?}"
    );
}
