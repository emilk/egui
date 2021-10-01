#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(enum_map::Enum)]
enum TokenType {
    Comment,
    Keyword,
    Literal,
    StringLiteral,
    Punctuation,
    Whitespace,
}

#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
struct CodeTheme {
    dark_mode: bool,
    formats: enum_map::EnumMap<TokenType, egui::TextFormat>,
}

impl Default for CodeTheme {
    fn default() -> Self {
        Self::dark(egui::TextStyle::Monospace)
    }
}

impl CodeTheme {
    fn dark(text_style: egui::TextStyle) -> Self {
        use egui::{Color32, TextFormat};
        Self {
            dark_mode: true,
            formats: enum_map::enum_map![
                TokenType::Comment => TextFormat::simple(text_style, Color32::from_gray(120)),
                TokenType::Keyword => TextFormat::simple(text_style, Color32::from_rgb(255, 100, 100)),
                TokenType::Literal => TextFormat::simple(text_style, Color32::from_rgb(178, 108, 210)),
                TokenType::StringLiteral => TextFormat::simple(text_style, Color32::from_rgb(109, 147, 226)),
                TokenType::Punctuation => TextFormat::simple(text_style, Color32::LIGHT_GRAY),
                TokenType::Whitespace => TextFormat::simple(text_style, Color32::TRANSPARENT),
            ],
        }
    }

    fn light(text_style: egui::TextStyle) -> Self {
        use egui::{Color32, TextFormat};
        Self {
            dark_mode: false,
            formats: enum_map::enum_map![
                TokenType::Comment => TextFormat::simple(text_style, Color32::GRAY),
                TokenType::Keyword => TextFormat::simple(text_style, Color32::from_rgb(235, 0, 0)),
                TokenType::Literal => TextFormat::simple(text_style, Color32::from_rgb(153, 134, 255)),
                TokenType::StringLiteral => TextFormat::simple(text_style, Color32::from_rgb(37, 203, 105)),
                TokenType::Punctuation => TextFormat::simple(text_style, Color32::DARK_GRAY),
                TokenType::Whitespace => TextFormat::simple(text_style, Color32::TRANSPARENT),
            ],
        }
    }
}

impl CodeTheme {
    fn ui(&mut self, ui: &mut egui::Ui, reset_value: CodeTheme) {
        ui.horizontal_top(|ui| {
            let mut selected_tt: TokenType = *ui.memory().data.get_or(TokenType::Comment);

            ui.vertical(|ui| {
                egui::widgets::global_dark_light_mode_buttons(ui);

                // ui.separator(); // TODO: fix forever-expand
                ui.add_space(14.0);

                ui.scope(|ui| {
                    for (tt, tt_name) in [
                        (TokenType::Comment, "// comment"),
                        (TokenType::Keyword, "keyword"),
                        (TokenType::Literal, "literal"),
                        (TokenType::StringLiteral, "\"string literal\""),
                        (TokenType::Punctuation, "punctuation ;"),
                        // (TokenType::Whitespace, "whitespace"),
                    ] {
                        let format = &mut self.formats[tt];
                        ui.style_mut().override_text_style = Some(format.style);
                        ui.visuals_mut().override_text_color = Some(format.color);
                        ui.radio_value(&mut selected_tt, tt, tt_name);
                    }
                });

                ui.add_space(14.0);

                if ui
                    .add(egui::Button::new("Reset theme").enabled(*self != reset_value))
                    .clicked()
                {
                    *self = reset_value;
                }
            });

            ui.add_space(16.0);
            // ui.separator(); // TODO: fix forever-expand

            ui.memory().data.insert(selected_tt);

            egui::Frame::group(ui.style())
                .margin(egui::Vec2::splat(2.0))
                .show(ui, |ui| {
                    // ui.group(|ui| {
                    ui.style_mut().override_text_style = Some(egui::TextStyle::Small);
                    ui.spacing_mut().slider_width = 128.0; // Controls color picker size
                    egui::widgets::color_picker::color_picker_color32(
                        ui,
                        &mut self.formats[selected_tt].color,
                        egui::color_picker::Alpha::Opaque,
                    );
                });
        });
    }
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct CodeEditor {
    theme_dark: CodeTheme,
    theme_light: CodeTheme,
    language: String,
    code: String,
    #[cfg_attr(feature = "serde", serde(skip))]
    highlighter: MemoizedSyntaxHighlighter,
}

impl Default for CodeEditor {
    fn default() -> Self {
        Self {
            theme_dark: CodeTheme::dark(egui::TextStyle::Monospace),
            theme_light: CodeTheme::light(egui::TextStyle::Monospace),
            language: "rs".into(),
            code: "// A very simple example\n\
fn main() {\n\
\tprintln!(\"Hello world!\");\n\
}\n\
"
            .into(),
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
            .default_height(500.0)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for CodeEditor {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let Self {
            theme_dark,
            theme_light,
            language,
            code,
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
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("Compile the demo with the ");
                ui.code("syntax_highlighting");
                ui.label(" feature to enable more accurate syntax highlighting using ");
                ui.hyperlink_to("syntect", "https://github.com/trishume/syntect");
                ui.label(".");
            });
        }

        ui.collapsing("Theme", |ui| {
            ui.group(|ui| {
                if ui.visuals().dark_mode {
                    let reset_value = CodeTheme::dark(egui::TextStyle::Monospace);
                    theme_dark.ui(ui, reset_value);
                } else {
                    let reset_value = CodeTheme::light(egui::TextStyle::Monospace);
                    theme_light.ui(ui, reset_value);
                }
            });
        });

        let theme = if ui.visuals().dark_mode {
            theme_dark
        } else {
            theme_light
        };

        let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
            let mut layout_job = highlighter.highlight(theme, string, language);
            layout_job.wrap_width = wrap_width;
            ui.fonts().layout_job(layout_job)
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(code)
                    .text_style(egui::TextStyle::Monospace) // for cursor height
                    .code_editor()
                    .desired_rows(10)
                    .lock_focus(true)
                    .desired_width(f32::INFINITY)
                    .layouter(&mut layouter),
            );
        });
    }
}

// ----------------------------------------------------------------------------

use egui::text::LayoutJob;

#[derive(Default)]
struct MemoizedSyntaxHighlighter {
    theme: CodeTheme,
    code: String,
    language: String,
    output: LayoutJob,
    highligher: Highligher,
}

impl MemoizedSyntaxHighlighter {
    fn highlight(&mut self, theme: &CodeTheme, code: &str, language: &str) -> LayoutJob {
        if (&self.theme, self.code.as_str(), self.language.as_str()) != (theme, code, language) {
            self.theme = *theme;
            self.code = code.to_owned();
            self.language = language.to_owned();
            self.output = self
                .highligher
                .highlight(theme, code, language)
                .unwrap_or_else(|| {
                    LayoutJob::simple(
                        code.into(),
                        egui::TextStyle::Monospace,
                        if theme.dark_mode {
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
    fn highlight(&self, theme: &CodeTheme, text: &str, language: &str) -> Option<LayoutJob> {
        use syntect::easy::HighlightLines;
        use syntect::highlighting::FontStyle;
        use syntect::util::LinesWithEndings;

        let syntax = self
            .ps
            .find_syntax_by_name(language)
            .or_else(|| self.ps.find_syntax_by_extension(language))?;

        let theme = if theme.dark_mode {
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
    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    fn highlight(&self, theme: &CodeTheme, mut text: &str, _language: &str) -> Option<LayoutJob> {
        // Extremely simple syntax highlighter for when we compile without syntect

        let mut job = LayoutJob::default();

        while !text.is_empty() {
            if text.starts_with("//") {
                let end = text.find('\n').unwrap_or_else(|| text.len());
                job.append(&text[..end], 0.0, theme.formats[TokenType::Comment]);
                text = &text[end..];
            } else if text.starts_with('"') {
                let end = text[1..]
                    .find('"')
                    .map(|i| i + 2)
                    .or_else(|| text.find('\n'))
                    .unwrap_or_else(|| text.len());
                job.append(&text[..end], 0.0, theme.formats[TokenType::StringLiteral]);
                text = &text[end..];
            } else if text.starts_with(|c: char| c.is_ascii_alphanumeric()) {
                let end = text[1..]
                    .find(|c: char| !c.is_ascii_alphanumeric())
                    .map(|i| i + 1)
                    .unwrap_or_else(|| text.len());
                let word = &text[..end];
                let tt = if is_keyword(word) {
                    TokenType::Keyword
                } else {
                    TokenType::Literal
                };
                job.append(word, 0.0, theme.formats[tt]);
                text = &text[end..];
            } else if text.starts_with(|c: char| c.is_ascii_whitespace()) {
                let end = text[1..]
                    .find(|c: char| !c.is_ascii_whitespace())
                    .map(|i| i + 1)
                    .unwrap_or_else(|| text.len());
                job.append(&text[..end], 0.0, theme.formats[TokenType::Whitespace]);
                text = &text[end..];
            } else {
                let mut it = text.char_indices();
                it.next();
                let end = it.next().map_or(text.len(), |(idx, _chr)| idx);
                job.append(&text[..end], 0.0, theme.formats[TokenType::Punctuation]);
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
