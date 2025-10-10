// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct CodeEditor {
    language: String,
    code: String,
}

impl Default for CodeEditor {
    fn default() -> Self {
        Self {
            language: "rs".into(),
            code: "// A very simple example\n\
fn main() {\n\
\tprintln!(\"Hello world!\");\n\
}\n\
"
            .into(),
        }
    }
}

impl crate::Demo for CodeEditor {
    fn name(&self) -> &'static str {
        "🖮 Code Editor"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        use crate::View as _;
        egui::Window::new(self.name())
            .open(open)
            .default_height(500.0)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| self.ui(ui));
    }
}

impl crate::View for CodeEditor {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let Self { language, code } = self;

        ui.horizontal(|ui| {
            ui.set_height(0.0);
            ui.label("An example of syntax highlighting in a TextEdit.");
            ui.add(crate::egui_github_link_file!());
        });

        if cfg!(feature = "syntect") {
            ui.horizontal(|ui| {
                ui.label("Language:");
                ui.text_edit_singleline(language);
            });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("Syntax highlighting powered by ");
                ui.hyperlink_to("syntect", "https://github.com/trishume/syntect");
                ui.label(".");
            });
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("Compile the demo with the ");
                ui.code("syntax_highlighting");
                ui.label(" feature to enable more accurate syntax highlighting using ");
                ui.hyperlink_to("syntect", "https://github.com/trishume/syntect");
                ui.label(".");
            });
        }

        let mut theme =
            egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());
        ui.collapsing("Theme", |ui| {
            ui.group(|ui| {
                theme.ui(ui);
                theme.clone().store_in_memory(ui.ctx());
            });
        });

        let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
            let mut layout_job = egui_extras::syntax_highlighting::highlight(
                ui.ctx(),
                ui.style(),
                &theme,
                buf.as_str(),
                language,
            );
            layout_job.wrap.max_width = wrap_width;
            ui.fonts_mut(|f| f.layout_job(layout_job))
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            let editor = egui::TextEdit::multiline(code)
                .font(egui::TextStyle::Monospace) // for cursor height
                .code_editor()
                .desired_rows(10)
                .lock_focus(true)
                .desired_width(f32::INFINITY)
                .layouter(&mut layouter);
            let editor = if cfg!(feature = "syntect") {
                editor
            } else {
                use egui::Color32;
                let background_color = if theme.is_dark() {
                    Color32::BLACK
                } else {
                    Color32::WHITE
                };
                editor.background_color(background_color)
            };
            ui.add(editor);
        });
    }
}
