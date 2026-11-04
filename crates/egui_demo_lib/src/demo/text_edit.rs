use egui::{Align, Align2, AtomExt as _};

/// Showcase [`egui::TextEdit`].
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TextEditDemo {
    pub text: String,
    /// `None` ("Auto") = the field's reading-start default, the HTML
    /// `dir=rtl`-input convention: the text/placeholder hug the reading start
    /// (left under a left-to-right interface, right under a right-to-left
    /// one). An explicit `Left`/`Center`/`Right` is physical, like
    /// `text-align` in HTML.
    halign: Option<egui::Align>,
    valign: egui::Align,
}

impl Default for TextEditDemo {
    fn default() -> Self {
        Self {
            text: crate::i18n::t("Edit this text").to_owned(),
            halign: None,
            valign: egui::Align::TOP,
        }
    }
}

impl crate::Demo for TextEditDemo {
    fn name(&self) -> &'static str {
        "✏️ TextEdit"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(crate::i18n::t(self.name()))
            .open(open)
            .resizable(false)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for TextEditDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });

        let Self {
            text,
            halign,
            valign,
        } = self;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label(crate::i18n::t("Advanced usage of "));
            ui.code("TextEdit");
            ui.label(".");
        });

        ui.horizontal(|ui| {
            ui.label(crate::i18n::t("Horizontal align:"));
            ui.selectable_value(halign, None, "Auto");
            ui.selectable_value(halign, Some(egui::Align::LEFT), crate::i18n::t("Left"));
            ui.selectable_value(halign, Some(egui::Align::Center), crate::i18n::t("Center"));
            ui.selectable_value(halign, Some(egui::Align::RIGHT), crate::i18n::t("Right"));
        });
        ui.horizontal(|ui| {
            ui.label(crate::i18n::t("Vertical align:"));
            ui.selectable_value(valign, egui::Align::TOP, crate::i18n::t("Top"));
            ui.selectable_value(valign, egui::Align::Center, crate::i18n::t("Center"));
            ui.selectable_value(valign, egui::Align::BOTTOM, crate::i18n::t("Bottom"));
        });

        let clear_id = egui::IdSalt::new("clear_button");
        let clear_size = egui::Vec2::splat(ui.spacing().interact_size.y);

        // Compose "Auto" here rather than skipping `.align`, because the
        // widget's fallback is an all-or-nothing `Align2` — dropping it would
        // also stop the vertical align radio from affecting the text. The
        // composition is geometry-identical to the widget's own style default
        // in both interface directions.
        let halign = (*halign).unwrap_or_else(|| {
            if ui.is_rtl() {
                egui::Align::RIGHT
            } else {
                egui::Align::LEFT
            }
        });

        let output = egui::TextEdit::multiline(text)
            .hint_text(crate::i18n::t("Type something!"))
            // Atoms are centered by default, so we need to pass the right align here:
            .prefix("🔎".atom_align(Align2::new(Align::LEFT, *valign)))
            .suffix(
                egui::Atom::custom(clear_id, clear_size)
                    .atom_align(Align2::new(Align::RIGHT, *valign)),
            )
            .align(Align2::new(halign, *valign))
            .show(ui);

        if let Some(rect) = output.response.rect(clear_id)
            && ui.place(rect, egui::Button::new("❌")).clicked()
        {
            text.clear();
        }

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label(crate::i18n::t("Selected text: "));
            if let Some(text_cursor_range) = output.cursor_range {
                let selected_text = text_cursor_range.slice_str(text);
                ui.code(selected_text);
            }
        });

        let anything_selected = output.cursor_range.is_some_and(|cursor| !cursor.is_empty());

        ui.add_enabled(
            anything_selected,
            egui::Label::new(crate::i18n::t(
                "Press ctrl+Y to toggle the case of selected text (cmd+Y on Mac)",
            )),
        );

        if output.response.has_focus()
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y))
            && let Some(text_cursor_range) = output.cursor_range
        {
            use egui::TextBuffer as _;
            let selected_chars = text_cursor_range.as_sorted_char_range();
            let selected_text = text.char_range(selected_chars.clone());
            let upper_case = selected_text.to_uppercase();
            let new_text = if selected_text == upper_case {
                selected_text.to_lowercase()
            } else {
                upper_case
            };
            text.delete_char_range(selected_chars.clone());
            text.insert_text(&new_text, selected_chars.start);
        }

        ui.horizontal(|ui| {
            ui.label(crate::i18n::t("Move cursor to the:"));

            if ui.button(crate::i18n::t("start")).clicked() {
                let text_edit_id = output.response.id;
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), text_edit_id) {
                    let ccursor = egui::text::CCursor::new(0);
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                    state.store(ui.ctx(), text_edit_id);
                    ui.memory_mut(|mem| mem.request_focus(text_edit_id)); // give focus back to the [`TextEdit`].
                }
            }

            if ui.button(crate::i18n::t("end")).clicked() {
                let text_edit_id = output.response.id;
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), text_edit_id) {
                    let ccursor = egui::text::CCursor::end_of_str(text);
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                    state.store(ui.ctx(), text_edit_id);
                    ui.memory_mut(|mem| mem.request_focus(text_edit_id)); // give focus back to the [`TextEdit`].
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use egui::epaint::Shape;
    use egui::epaint::text::TextDirection;
    use egui::{CentralPanel, Context, FullOutput, Key, Modifiers, Pos2, RawInput, Rect, Ui, Vec2};
    use egui::{accesskit, vec2};
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable as _;

    use super::TextEditDemo;
    use crate::demo::Demo as _;

    #[test]
    pub fn should_type() {
        let text = "Hello, world!".to_owned();
        let mut harness = Harness::new_ui_state(
            move |ui, text| {
                CentralPanel::default().show(ui, |ui| {
                    let label = ui.label("Text:");
                    ui.text_edit_singleline(text).labelled_by(label.id);
                });
            },
            text,
        );

        harness.run();

        let text_edit = harness.get_by_role(accesskit::Role::TextInput);
        assert_eq!(text_edit.value().as_deref(), Some("Hello, world!"));
        text_edit.focus();

        harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
        text_edit.type_text("Hi ");

        harness.run();
        harness
            .get_by_role(accesskit::Role::TextInput)
            .type_text("there!");

        harness.run();
        let text_edit = harness.get_by_role(accesskit::Role::TextInput);
        assert_eq!(text_edit.value().as_deref(), Some("Hi there!"));
        assert_eq!(harness.state(), "Hi there!");
    }

    // -------------------------------------------------------------------------
    // The demo's default alignment: `Auto` (= the field's reading-start default)
    // must anchor the placeholder at the reading start like the widget's own
    // default, in both interface directions. The 🔎 prefix atom is the stable
    // anchor: the RTL atom-row reversal always places it in the row's
    // reading-start cell regardless of field width, so only RELATIVE adjacency
    // is asserted — never absolute pixels (the demo window is user-resizable).

    const SCREEN: Vec2 = vec2(400.0, 600.0);

    /// Slack for the atom gap and glyph bearings around an "adjacent across the
    /// reading direction" assertion.
    const EDGE_TOL: f32 = 12.0;

    fn input(size: Vec2, time: f64) -> RawInput {
        RawInput {
            time: Some(time),
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            ..Default::default()
        }
    }

    /// Run the ui code until the demo window's fade-in settles, discarding the
    /// texture deltas (these tests never render). egui areas fade in over
    /// `animation_time` starting when they first appear, so a single first pass
    /// paints every shape fully transparent ([`Shape::Noop`]); the last pass is
    /// the settled, fully-opaque one.
    fn run(ctx: &Context, mut run_ui: impl FnMut(&mut Ui)) -> FullOutput {
        let mut output = None;
        for i in 0..8 {
            let mut out = ctx.run_ui(input(SCREEN, i as f64 * 0.1), &mut run_ui);
            out.textures_delta.clear();
            output = Some(out);
        }
        output.unwrap_or_else(|| panic!("no pass ran"))
    }

    /// (job text, absolute layout rect, resolved direction) for every text shape
    /// painted this pass.
    fn painted_texts(output: &FullOutput) -> Vec<(String, Rect, Option<TextDirection>)> {
        fn collect(shape: &Shape, out: &mut Vec<(String, Rect, Option<TextDirection>)>) {
            match shape {
                Shape::Text(text) => {
                    out.push((
                        text.galley.job.text.clone(),
                        text.galley.rect.translate(text.pos.to_vec2()),
                        text.galley.job.direction,
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

    /// The ink rect and resolved direction of the painted text whose galley job
    /// text equals `needle` (the input value, or the hint text while the field
    /// is empty).
    fn painted_text(output: &FullOutput, needle: &str) -> (Rect, Option<TextDirection>) {
        painted_texts(output)
            .into_iter()
            .find(|(text, _, _)| text == needle)
            .map(|(_, rect, direction)| (rect, direction))
            .unwrap_or_else(|| panic!("no painted text galley for {needle:?}"))
    }

    /// Show the demo in its default state (value cleared, so the hint paints)
    /// under the given interface direction, and return
    /// (hint ink, hint direction, prefix ink).
    fn demo_hint_geometry(rtl: bool) -> (Rect, Option<TextDirection>, Rect) {
        let mut demo = TextEditDemo::default();
        // The demo's default value would suppress the hint; the reported state
        // is an empty field:
        demo.text.clear();
        let ctx = Context::default();
        ctx.set_right_to_left(rtl);
        let mut open = true;
        let output = run(&ctx, |ui| {
            demo.show(ui, &mut open);
        });
        let (hint_ink, hint_direction) = painted_text(&output, "Type something!");
        let (prefix_ink, _) = painted_text(&output, "🔎");
        (hint_ink, hint_direction, prefix_ink)
    }

    /// Under a right-to-left interface the demo's default state must anchor the
    /// placeholder at the reading start: the hint's reading-start (right) edge
    /// sits adjacent to the 🔎 prefix's reading-end (left) edge — the same
    /// geometry the widget's own direction-aware default produces. (Before the
    /// demo's `Auto` default, the explicit `Align::LEFT` pinned the hint at the
    /// physical left, more than a hundred pixels away from the prefix — the
    /// failure message prints the measured distance.)
    #[test]
    fn rtl_demo_text_defaults_to_the_reading_start() {
        let (hint_ink, hint_direction, prefix_ink) = demo_hint_geometry(true);
        assert_eq!(
            hint_direction,
            Some(TextDirection::RightToLeft),
            "the hint's base direction should follow the interface (hint {hint_ink:?})"
        );
        // Reading start of the hint is its RIGHT edge; reading end of the
        // prefix is its LEFT edge:
        let gap = prefix_ink.left() - hint_ink.right();
        assert!(
            gap.abs() <= EDGE_TOL,
            "the hint should sit adjacent to the prefix across the reading \
             direction (gap {gap}, hint {hint_ink:?}, prefix {prefix_ink:?})"
        );
    }

    /// LTR twin / unchanged guard: under the default interface the placeholder
    /// sits adjacent to the prefix on the left, with a left-to-right base
    /// direction.
    #[test]
    fn ltr_demo_text_defaults_left() {
        let (hint_ink, hint_direction, prefix_ink) = demo_hint_geometry(false);
        assert_eq!(
            hint_direction,
            Some(TextDirection::LeftToRight),
            "the hint's base direction should follow the interface (hint {hint_ink:?})"
        );
        // Reading start of the hint is its LEFT edge; reading end of the
        // prefix is its RIGHT edge:
        let gap = hint_ink.left() - prefix_ink.right();
        assert!(
            gap.abs() <= EDGE_TOL,
            "the hint should sit adjacent to the prefix across the reading \
             direction (gap {gap}, hint {hint_ink:?}, prefix {prefix_ink:?})"
        );
    }
}
