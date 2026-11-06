//! Regression tests for the UI language plumbing:
//! [`egui::Context::set_language`]/[`egui::Context::language`],
//! [`egui::UiBuilder::language`]/[`egui::Ui::language`], and the widget
//! stamping of `LayoutJob::language`.
//!
//! No behaviour change for `language: None` is also asserted here:
//! the default context language is `None` and setting a language changes no
//! layout geometry.

use egui::{
    CentralPanel, Context, FontSelection, Pos2, RawInput, Rect, RichText, TextWrapMode, Ui,
    UiBuilder, Vec2, WidgetText, vec2,
};

fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        ..Default::default()
    }
}

/// Run one ui pass inside a central panel and discard the texture deltas
/// (these tests never render). Returns what the closure returned.
fn run<R>(ctx: &Context, size: Vec2, run_ui: impl FnOnce(&mut Ui) -> R + 'static) -> R {
    let mut run_ui = Some(run_ui);
    let mut result = None;
    let mut output = ctx.run_ui(input(size), |ui| {
        CentralPanel::default().show(ui, |ui| {
            if let Some(run_ui) = run_ui.take() {
                result = Some(run_ui(ui));
            }
        });
    });
    output.textures_delta.clear();
    result.expect("the ui closure always runs")
}

/// The context language is stamped onto widget text from every
/// job-building path: `WidgetText::Text`, `WidgetText::RichText`, `Label`,
/// and `TextEdit`.
#[test]
fn context_language_stamps_widget_text() {
    let ctx = Context::default();
    ctx.set_language("th");

    run(&ctx, vec2(400.0, 300.0), |ui| {
        let galley = WidgetText::from("ภาษาไทย").into_galley(
            ui,
            Some(TextWrapMode::Wrap),
            200.0,
            FontSelection::Default,
        );
        assert_eq!(
            galley.job.language,
            Some("th".to_owned()),
            "the Text variant should be stamped with the context language"
        );

        let galley = WidgetText::from(RichText::new("ภาษาไทย")).into_galley(
            ui,
            Some(TextWrapMode::Wrap),
            200.0,
            FontSelection::Default,
        );
        assert_eq!(
            galley.job.language,
            Some("th".to_owned()),
            "the RichText variant should be stamped with the context language"
        );

        let (_pos, galley, _response) = egui::Label::new("thai").layout_in_ui(ui);
        assert_eq!(
            galley.job.language,
            Some("th".to_owned()),
            "Label should be stamped with the context language"
        );

        let mut text = String::new();
        let output = egui::TextEdit::singleline(&mut text).show(ui);
        assert_eq!(
            output.galley.job.language,
            Some("th".to_owned()),
            "TextEdit should be stamped with the context language"
        );
    });
}

/// A `UiBuilder::language` override wins over the context language,
/// is inherited by nested scopes, and does not leak into sibling scopes.
#[test]
fn ui_builder_language_overrides_subtree_and_inherits() {
    let ctx = Context::default();
    ctx.set_language("th");

    run(&ctx, vec2(400.0, 300.0), |ui| {
        let response = ui.scope_builder(UiBuilder::new().language("ja"), |ui| {
            assert_eq!(ui.language(), Some("ja".to_owned()));

            let (_pos, galley, _r) = egui::Label::new("japanese").layout_in_ui(ui);
            assert_eq!(
                galley.job.language,
                Some("ja".to_owned()),
                "a label inside the override subtree should get the override language"
            );

            // A nested scope inherits the override:
            ui.scope(|ui| {
                assert_eq!(ui.language(), Some("ja".to_owned()), "inheritance");
            });
            "ja".to_owned()
        });
        assert_eq!(response.inner, "ja".to_owned());

        // A sibling scope after the override still gets the context language:
        ui.scope(|ui| {
            assert_eq!(ui.language(), Some("th".to_owned()), "sibling");
        });
    });
}

/// An explicit `LayoutJob::language` wins over the ambient language,
/// both through `WidgetText::into_galley` and through `Label`.
#[test]
fn explicit_job_language_wins_over_ambient() {
    let ctx = Context::default();
    ctx.set_language("th");

    run(&ctx, vec2(400.0, 300.0), |ui| {
        let mut job = egui::text::LayoutJob::simple(
            "텍스트".into(),
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
            200.0,
        );
        job.language = Some("ko".into());

        let galley = WidgetText::from(job.clone()).into_galley(
            ui,
            Some(TextWrapMode::Wrap),
            200.0,
            FontSelection::Default,
        );
        assert_eq!(
            galley.job.language,
            Some("ko".to_owned()),
            "the explicit job language must win over the ambient one"
        );

        let (_pos, galley, _response) = egui::Label::new(job).layout_in_ui(ui);
        assert_eq!(
            galley.job.language,
            Some("ko".to_owned()),
            "Label must not overwrite an explicit job language"
        );
    });
}

/// The language is `None` by default, and setting one changes no layout
/// geometry: two galleys of the same text with and without a context language
/// have equal rows and rect.
#[test]
fn language_is_none_by_default_and_does_not_change_layout() {
    let ctx = Context::default();
    run(&ctx, vec2(400.0, 300.0), |ui| {
        assert!(
            ui.language().is_none(),
            "the default context language is None"
        );

        let (_pos, galley, _r) = egui::Label::new("plain text").layout_in_ui(ui);
        assert_eq!(galley.job.language, None);
    });

    let with_language = Context::default();
    with_language.set_language("th");
    let without = Context::default();

    let (plain_rows, plain_rect) = run(&without, vec2(400.0, 300.0), |ui| {
        let galley = WidgetText::from("same text").into_galley(
            ui,
            Some(TextWrapMode::Wrap),
            200.0,
            FontSelection::Default,
        );
        (galley.rows.clone(), galley.rect)
    });
    let (thai_rows, thai_rect) = run(&with_language, vec2(400.0, 300.0), |ui| {
        let galley = WidgetText::from("same text").into_galley(
            ui,
            Some(TextWrapMode::Wrap),
            200.0,
            FontSelection::Default,
        );
        (galley.rows.clone(), galley.rect)
    });

    assert_eq!(plain_rows, thai_rows, "language alone changes no rows");
    assert_eq!(plain_rect, thai_rect, "language alone changes no rect");
}
