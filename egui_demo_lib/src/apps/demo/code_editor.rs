use egui::text::LayoutJob;

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct CodeEditor {
    code: String,
    language: String,
    #[cfg_attr(feature = "persistence", serde(skip))]
    highlighter: MemoizedSyntaxHighlighter,
}

impl Default for CodeEditor {
    fn default() -> Self {
        Self {
            code: "// A very simple example\n\
fn main() {\n\
\tprintln!(\"Hello world!\");\n\
}\n\
"
            .into(),
            language: "rs".into(),
            highlighter: Default::default(),
        }
    }
}

impl super::Demo for CodeEditor {
    fn name(&self) -> &'static str {
        "🖮 Code Editor"
    }

    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool) {
        use super::View;
        egui::Window::new(self.name())
            .open(open)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for CodeEditor {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let Self {
            code,
            language,
            highlighter,
        } = self;

        ui.horizontal(|ui| {
            ui.set_height(0.0);
            ui.label("An example of syntax highlighting in a TextEdit.");
            ui.add(crate::__egui_github_link_file!());
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
            ui.horizontal_wrapped(|ui|{
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("Compile the demo with the 'syntax_highlighting' feature to enable much nicer syntax highlighting using ");
                ui.hyperlink_to("syntect", "https://github.com/trishume/syntect");
                ui.label(".");
            });
        }

        let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
            let mut layout_job = highlighter.highlight(ui.visuals().dark_mode, string, language);
            layout_job.wrap_width = wrap_width;
            ui.fonts().layout_job(layout_job)
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(code)
                    .text_style(egui::TextStyle::Monospace) // for cursor height
                    .code_editor()
                    .lock_focus(true)
                    .desired_width(f32::INFINITY)
                    .layouter(&mut layouter),
            );
        });
    }
}

// ----------------------------------------------------------------------------

#[derive(Default)]
struct MemoizedSyntaxHighlighter {
    is_dark_mode: bool,
    code: String,
    language: String,
    output: LayoutJob,
    highligher: Highligher,
}

impl MemoizedSyntaxHighlighter {
    fn highlight(&mut self, is_dark_mode: bool, code: &str, language: &str) -> LayoutJob {
        if (
            self.is_dark_mode,
            self.code.as_str(),
            self.language.as_str(),
        ) != (is_dark_mode, code, language)
        {
            self.is_dark_mode = is_dark_mode;
            self.code = code.to_owned();
            self.language = language.to_owned();
            self.output = self
                .highligher
                .highlight(is_dark_mode, code, language)
                .unwrap_or_else(|| {
                    LayoutJob::simple(
                        code.into(),
                        egui::TextStyle::Monospace,
                        if is_dark_mode {
                            egui::Color32::LIGHT_GRAY
                        } else {
                            egui::Color32::DARK_GRAY
                        },
                        f32::INFINITY,
                    )
                });
        }
        self.output.clone()
    }
}

// ----------------------------------------------------------------------------

#[cfg(feature = "syntect")]
struct Highligher {
    ps: syntect::parsing::SyntaxSet,
    ts: syntect::highlighting::ThemeSet,
}

#[cfg(feature = "syntect")]
impl Default for Highligher {
    fn default() -> Self {
        Self {
            ps: syntect::parsing::SyntaxSet::load_defaults_newlines(),
            ts: syntect::highlighting::ThemeSet::load_defaults(),
        }
    }
}

#[cfg(feature = "syntect")]
impl Highligher {
    fn highlight(&self, is_dark_mode: bool, text: &str, language: &str) -> Option<LayoutJob> {
        use syntect::easy::HighlightLines;
        use syntect::highlighting::FontStyle;
        use syntect::util::LinesWithEndings;

        let syntax = self
            .ps
            .find_syntax_by_name(language)
            .or_else(|| self.ps.find_syntax_by_extension(language))?;

        let theme = if is_dark_mode {
            "base16-mocha.dark"
        } else {
            "base16-ocean.light"
        };
        let mut h = HighlightLines::new(syntax, &self.ts.themes[theme]);

        use egui::text::{LayoutSection, TextFormat};

        let mut job = LayoutJob {
            text: text.into(),
            ..Default::default()
        };

        for line in LinesWithEndings::from(text) {
            for (style, range) in h.highlight(line, &self.ps) {
                let fg = style.foreground;
                let text_color = egui::Color32::from_rgb(fg.r, fg.g, fg.b);
                let italics = style.font_style.contains(FontStyle::ITALIC);
                let underline = style.font_style.contains(FontStyle::ITALIC);
                let underline = if underline {
                    egui::Stroke::new(1.0, text_color)
                } else {
                    egui::Stroke::none()
                };
                job.sections.push(LayoutSection {
                    leading_space: 0.0,
                    byte_range: as_byte_range(text, range),
                    format: TextFormat {
                        style: egui::TextStyle::Monospace,
                        color: text_color,
                        italics,
                        underline,
                        ..Default::default()
                    },
                });
            }
        }

        Some(job)
    }
}

#[cfg(feature = "syntect")]
fn as_byte_range(whole: &str, range: &str) -> std::ops::Range<usize> {
    let whole_start = whole.as_ptr() as usize;
    let range_start = range.as_ptr() as usize;
    assert!(whole_start <= range_start);
    assert!(range_start + range.len() <= whole_start + whole.len());
    let offset = range_start - whole_start;
    offset..(offset + range.len())
}

// ----------------------------------------------------------------------------

#[cfg(not(feature = "syntect"))]
#[derive(Default)]
struct Highligher {}

#[cfg(not(feature = "syntect"))]
impl Highligher {
    fn highlight(&self, is_dark_mode: bool, mut text: &str, _language: &str) -> Option<LayoutJob> {
        // Extremely simple syntax highlighter for when we compile without syntect

        use egui::text::TextFormat;
        use egui::Color32;
        let monospace = egui::TextStyle::Monospace;

        let comment_format = TextFormat::simple(monospace, Color32::GRAY);
        let quoted_string_format = TextFormat::simple(
            monospace,
            if is_dark_mode {
                Color32::KHAKI
            } else {
                Color32::BROWN
            },
        );
        let keyword_format = TextFormat::simple(
            monospace,
            if is_dark_mode {
                Color32::LIGHT_RED
            } else {
                Color32::DARK_RED
            },
        );
        let literal_format = TextFormat::simple(
            monospace,
            if is_dark_mode {
                Color32::LIGHT_GREEN
            } else {
                Color32::DARK_GREEN
            },
        );
        let whitespace_format = TextFormat::simple(monospace, Color32::WHITE);
        let punctuation_format = TextFormat::simple(
            monospace,
            if is_dark_mode {
                Color32::LIGHT_GRAY
            } else {
                Color32::DARK_GRAY
            },
        );

        let mut job = LayoutJob::default();

        while !text.is_empty() {
            if text.starts_with("//") {
                let end = text.find('\n').unwrap_or(text.len());
                job.append(&text[..end], 0.0, comment_format);
                text = &text[end..];
            } else if text.starts_with('"') {
                let end = text[1..]
                    .find('"')
                    .map(|i| i + 2)
                    .or_else(|| text.find('\n'))
                    .unwrap_or(text.len());
                job.append(&text[..end], 0.0, quoted_string_format);
                text = &text[end..];
            } else if text.starts_with(|c: char| c.is_ascii_alphanumeric()) {
                let end = text[1..]
                    .find(|c: char| !c.is_ascii_alphanumeric())
                    .map(|i| i + 1)
                    .unwrap_or(text.len());
                let word = &text[..end];
                if is_keyword(word) {
                    job.append(word, 0.0, keyword_format);
                } else {
                    job.append(word, 0.0, literal_format);
                };
                text = &text[end..];
            } else if text.starts_with(|c: char| c.is_ascii_whitespace()) {
                let end = text[1..]
                    .find(|c: char| !c.is_ascii_whitespace())
                    .map(|i| i + 1)
                    .unwrap_or(text.len());
                job.append(&text[..end], 0.0, whitespace_format);
                text = &text[end..];
            } else {
                let mut it = text.char_indices();
                it.next();
                let end = it.next().map_or(text.len(), |(idx, _chr)| idx);
                job.append(&text[..end], 0.0, punctuation_format);
                text = &text[end..];
            }
        }

        Some(job)
    }
}

#[cfg(not(feature = "syntect"))]
fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
    )
}
