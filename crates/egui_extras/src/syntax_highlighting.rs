//! Syntax highlighting for code.
//!
//! Turn on the `syntect` feature for great syntax highlighting of any language.
//! Otherwise, a very simple fallback will be used, that works okish for C, C++, Rust, and Python.

use egui::TextStyle;
use egui::text::LayoutJob;

/// View some code with syntax highlighting and selection.
pub fn code_view_ui(
    ui: &mut egui::Ui,
    theme: &CodeTheme,
    code: &str,
    language: &str,
) -> egui::Response {
    let layout_job = highlight(ui.ctx(), ui.style(), theme, code, language);
    ui.add(egui::Label::new(layout_job).selectable(true))
}

/// Add syntax highlighting to a code string.
///
/// The results are memoized, so you can call this every frame without performance penalty.
pub fn highlight(
    ctx: &egui::Context,
    style: &egui::Style,
    theme: &CodeTheme,
    code: &str,
    language: &str,
) -> LayoutJob {
    highlight_inner(ctx, style, theme, code, language, None)
}

/// Add syntax highlighting to a code string, with custom `syntect` settings
///
/// The results are memoized, so you can call this every frame without performance penalty.
///
/// The `syntect` settings are memoized by *address*, so a stable reference should
/// be used to avoid unnecessary recomputation.
#[cfg(feature = "syntect")]
pub fn highlight_with(
    ctx: &egui::Context,
    style: &egui::Style,
    theme: &CodeTheme,
    code: &str,
    language: &str,
    settings: &SyntectSettings,
) -> LayoutJob {
    highlight_inner(
        ctx,
        style,
        theme,
        code,
        language,
        Some(HighlightSettings(settings)),
    )
}

fn highlight_inner(
    ctx: &egui::Context,
    style: &egui::Style,
    theme: &CodeTheme,
    code: &str,
    language: &str,
    settings: Option<HighlightSettings<'_>>,
) -> LayoutJob {
    // We take in both context and style so that in situations where ui is not available such as when
    // performing it at a separate thread (ctx, ctx.global_style()) can be used and when ui is available
    // (ui.ctx(), ui.style()) can be used

    #[expect(non_local_definitions)]
    impl
        egui::cache::ComputerMut<
            (&egui::FontId, &CodeTheme, &str, &str, HighlightSettings<'_>),
            LayoutJob,
        > for Highlighter
    {
        fn compute(
            &mut self,
            (font_id, theme, code, lang, settings): (
                &egui::FontId,
                &CodeTheme,
                &str,
                &str,
                HighlightSettings<'_>,
            ),
        ) -> LayoutJob {
            Self::highlight(font_id.clone(), theme, code, lang, settings)
        }
    }

    type HighlightCache = egui::cache::FrameCache<LayoutJob, Highlighter>;

    let font_id = style
        .override_font_id
        .clone()
        .unwrap_or_else(|| TextStyle::Monospace.resolve(style));

    // Private type, so that users can't interfere with it in the `IdTypeMap`
    #[cfg(feature = "syntect")]
    #[derive(Clone, Default)]
    struct PrivateSettings(std::sync::Arc<SyntectSettings>);

    // Dummy private settings, to minimize code changes without `syntect`
    #[cfg(not(feature = "syntect"))]
    #[derive(Clone, Default)]
    struct PrivateSettings(std::sync::Arc<()>);

    ctx.memory_mut(|mem| {
        let settings = settings.unwrap_or_else(|| {
            HighlightSettings(
                &mem.data
                    .get_temp_mut_or_default::<PrivateSettings>(egui::Id::NULL)
                    .0,
            )
        });
        mem.caches
            .cache::<HighlightCache>()
            .get((&font_id, theme, code, language, settings))
            .clone()
    })
}

fn monospace_font_size(style: &egui::Style) -> f32 {
    TextStyle::Monospace.resolve(style).size
}

// ----------------------------------------------------------------------------

#[cfg(not(feature = "syntect"))]
#[derive(Clone, Copy, PartialEq, enum_map::Enum)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum TokenType {
    Comment,
    Keyword,
    Literal,
    StringLiteral,
    Punctuation,
    Whitespace,
}

#[cfg(feature = "syntect")]
#[derive(Clone, Copy, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum SyntectTheme {
    Base16EightiesDark,
    Base16MochaDark,
    Base16OceanDark,
    Base16OceanLight,
    InspiredGitHub,
    SolarizedDark,
    SolarizedLight,
}

#[cfg(feature = "syntect")]
impl SyntectTheme {
    fn all() -> impl ExactSizeIterator<Item = Self> {
        [
            Self::Base16EightiesDark,
            Self::Base16MochaDark,
            Self::Base16OceanDark,
            Self::Base16OceanLight,
            Self::InspiredGitHub,
            Self::SolarizedDark,
            Self::SolarizedLight,
        ]
        .iter()
        .copied()
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Base16EightiesDark => "Base16 Eighties (dark)",
            Self::Base16MochaDark => "Base16 Mocha (dark)",
            Self::Base16OceanDark => "Base16 Ocean (dark)",
            Self::Base16OceanLight => "Base16 Ocean (light)",
            Self::InspiredGitHub => "InspiredGitHub (light)",
            Self::SolarizedDark => "Solarized (dark)",
            Self::SolarizedLight => "Solarized (light)",
        }
    }

    fn syntect_key_name(&self) -> &'static str {
        match self {
            Self::Base16EightiesDark => "base16-eighties.dark",
            Self::Base16MochaDark => "base16-mocha.dark",
            Self::Base16OceanDark => "base16-ocean.dark",
            Self::Base16OceanLight => "base16-ocean.light",
            Self::InspiredGitHub => "InspiredGitHub",
            Self::SolarizedDark => "Solarized (dark)",
            Self::SolarizedLight => "Solarized (light)",
        }
    }

    pub fn is_dark(&self) -> bool {
        match self {
            Self::Base16EightiesDark
            | Self::Base16MochaDark
            | Self::Base16OceanDark
            | Self::SolarizedDark => true,

            Self::Base16OceanLight | Self::InspiredGitHub | Self::SolarizedLight => false,
        }
    }
}

/// A selected color theme.
#[derive(Clone, Hash, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(default)
)]
pub struct CodeTheme {
    dark_mode: bool,

    #[cfg(feature = "syntect")]
    syntect_theme: SyntectTheme,
    #[cfg(feature = "syntect")]
    font_id: egui::FontId,

    #[cfg(not(feature = "syntect"))]
    formats: enum_map::EnumMap<TokenType, egui::TextFormat>,
}

impl Default for CodeTheme {
    fn default() -> Self {
        Self::dark(12.0)
    }
}

impl CodeTheme {
    pub fn is_dark(&self) -> bool {
        self.dark_mode
    }

    /// Selects either dark or light theme based on the given style.
    pub fn from_style(style: &egui::Style) -> Self {
        let font_id = style
            .override_font_id
            .clone()
            .unwrap_or_else(|| TextStyle::Monospace.resolve(style));

        if style.visuals.dark_mode {
            Self::dark_with_font_id(font_id)
        } else {
            Self::light_with_font_id(font_id)
        }
    }

    /// ### Example
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// use egui_extras::syntax_highlighting::CodeTheme;
    /// let theme = CodeTheme::dark(12.0);
    /// # });
    /// ```
    pub fn dark(font_size: f32) -> Self {
        Self::dark_with_font_id(egui::FontId::monospace(font_size))
    }

    /// ### Example
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// use egui_extras::syntax_highlighting::CodeTheme;
    /// let theme = CodeTheme::light(12.0);
    /// # });
    /// ```
    pub fn light(font_size: f32) -> Self {
        Self::light_with_font_id(egui::FontId::monospace(font_size))
    }

    /// Load code theme from egui memory.
    ///
    /// There is one dark and one light theme stored at any one time.
    pub fn from_memory(ctx: &egui::Context, style: &egui::Style) -> Self {
        #![expect(clippy::needless_return)]

        let (id, default) = if style.visuals.dark_mode {
            (egui::Id::new("dark"), Self::dark as fn(f32) -> Self)
        } else {
            (egui::Id::new("light"), Self::light as fn(f32) -> Self)
        };

        #[cfg(feature = "serde")]
        {
            return ctx.data_mut(|d| {
                d.get_persisted(id)
                    .unwrap_or_else(|| default(monospace_font_size(style)))
            });
        }

        #[cfg(not(feature = "serde"))]
        {
            return ctx.data_mut(|d| {
                d.get_temp(id)
                    .unwrap_or_else(|| default(monospace_font_size(style)))
            });
        }
    }

    /// Store theme to egui memory.
    ///
    /// There is one dark and one light theme stored at any one time.
    pub fn store_in_memory(self, ctx: &egui::Context) {
        let id = if ctx.global_style().visuals.dark_mode {
            egui::Id::new("dark")
        } else {
            egui::Id::new("light")
        };

        #[cfg(feature = "serde")]
        ctx.data_mut(|d| d.insert_persisted(id, self));

        #[cfg(not(feature = "serde"))]
        ctx.data_mut(|d| d.insert_temp(id, self));
    }
}

#[cfg(feature = "syntect")]
impl CodeTheme {
    /// Change the font size
    pub fn with_font_size(&self, font_size: f32) -> Self {
        Self {
            dark_mode: self.dark_mode,
            syntect_theme: self.syntect_theme,
            font_id: egui::FontId::monospace(font_size),
        }
    }

    /// Change the `font_id` of the theme
    pub fn with_font_id(&self, font_id: egui::FontId) -> Self {
        Self {
            dark_mode: self.dark_mode,
            syntect_theme: self.syntect_theme,
            font_id,
        }
    }

    fn dark_with_font_id(font_id: egui::FontId) -> Self {
        Self {
            dark_mode: true,
            syntect_theme: SyntectTheme::Base16MochaDark,
            font_id,
        }
    }

    fn light_with_font_id(font_id: egui::FontId) -> Self {
        Self {
            dark_mode: false,
            syntect_theme: SyntectTheme::SolarizedLight,
            font_id,
        }
    }

    /// Show UI for changing the color theme.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.dark_mode, true, "🌙 Dark theme")
                .on_hover_text("Use the dark mode theme");

            ui.selectable_value(&mut self.dark_mode, false, "☀ Light theme")
                .on_hover_text("Use the light mode theme");
        });
        let current_theme_is_dark = self.is_dark();
        for theme in SyntectTheme::all().filter(|t| t.is_dark() == current_theme_is_dark) {
            ui.radio_value(&mut self.syntect_theme, theme, theme.name());
        }
    }
}

#[cfg(not(feature = "syntect"))]
impl CodeTheme {
    // The syntect version takes it by value. This could be avoided by specializing the from_style
    // function, but at the cost of more code duplication.
    #[expect(clippy::needless_pass_by_value)]
    fn dark_with_font_id(font_id: egui::FontId) -> Self {
        #![expect(clippy::mem_forget)]
        use egui::{Color32, TextFormat};
        Self {
            dark_mode: true,
            formats: enum_map::enum_map![
                TokenType::Comment => TextFormat::simple(font_id.clone(), Color32::from_gray(120)),
                TokenType::Keyword => TextFormat::simple(font_id.clone(), Color32::from_rgb(255, 100, 100)),
                TokenType::Literal => TextFormat::simple(font_id.clone(), Color32::from_rgb(87, 165, 171)),
                TokenType::StringLiteral => TextFormat::simple(font_id.clone(), Color32::from_rgb(109, 147, 226)),
                TokenType::Punctuation => TextFormat::simple(font_id.clone(), Color32::LIGHT_GRAY),
                TokenType::Whitespace => TextFormat::simple(font_id.clone(), Color32::TRANSPARENT),
            ],
        }
    }

    // The syntect version takes it by value
    #[expect(clippy::needless_pass_by_value)]
    fn light_with_font_id(font_id: egui::FontId) -> Self {
        #![expect(clippy::mem_forget)]
        use egui::{Color32, TextFormat};
        Self {
            dark_mode: false,
            #[cfg(not(feature = "syntect"))]
            formats: enum_map::enum_map![
                TokenType::Comment => TextFormat::simple(font_id.clone(), Color32::GRAY),
                TokenType::Keyword => TextFormat::simple(font_id.clone(), Color32::from_rgb(235, 0, 0)),
                TokenType::Literal => TextFormat::simple(font_id.clone(), Color32::from_rgb(153, 134, 255)),
                TokenType::StringLiteral => TextFormat::simple(font_id.clone(), Color32::from_rgb(37, 203, 105)),
                TokenType::Punctuation => TextFormat::simple(font_id.clone(), Color32::DARK_GRAY),
                TokenType::Whitespace => TextFormat::simple(font_id.clone(), Color32::TRANSPARENT),
            ],
        }
    }

    /// Show UI for changing the color theme.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_top(|ui| {
            let selected_id = egui::Id::NULL;

            #[cfg(feature = "serde")]
            let mut selected_tt: TokenType =
                ui.data_mut(|d| *d.get_persisted_mut_or(selected_id, TokenType::Comment));

            #[cfg(not(feature = "serde"))]
            let mut selected_tt: TokenType =
                ui.data_mut(|d| *d.get_temp_mut_or(selected_id, TokenType::Comment));

            ui.vertical(|ui| {
                ui.set_width(150.0);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.dark_mode, true, "🌙 Dark theme")
                        .on_hover_text("Use the dark mode theme");

                    ui.selectable_value(&mut self.dark_mode, false, "☀ Light theme")
                        .on_hover_text("Use the light mode theme");
                });
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
                        ui.style_mut().override_font_id = Some(format.font_id.clone());
                        ui.visuals_mut().override_text_color = Some(format.color);
                        ui.radio_value(&mut selected_tt, tt, tt_name);
                    }
                });

                let reset_value = if self.dark_mode {
                    Self::dark(monospace_font_size(ui.style()))
                } else {
                    Self::light(monospace_font_size(ui.style()))
                };

                if ui
                    .add_enabled(*self != reset_value, egui::Button::new("Reset theme"))
                    .clicked()
                {
                    *self = reset_value;
                }
            });

            ui.add_space(16.0);

            #[cfg(feature = "serde")]
            ui.data_mut(|d| d.insert_persisted(selected_id, selected_tt));
            #[cfg(not(feature = "serde"))]
            ui.data_mut(|d| d.insert_temp(selected_id, selected_tt));

            egui::Frame::group(ui.style())
                .inner_margin(egui::Vec2::splat(2.0))
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

#[cfg(feature = "syntect")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SyntectSettings {
    pub ps: syntect::parsing::SyntaxSet,
    pub ts: syntect::highlighting::ThemeSet,
}

#[cfg(feature = "syntect")]
impl Default for SyntectSettings {
    fn default() -> Self {
        profiling::function_scope!();
        Self {
            ps: syntect::parsing::SyntaxSet::load_defaults_newlines(),
            ts: syntect::highlighting::ThemeSet::load_defaults(),
        }
    }
}

/// Highlight settings are memoized by reference address, rather than value
#[cfg(feature = "syntect")]
#[derive(Copy, Clone)]
struct HighlightSettings<'a>(&'a SyntectSettings);

#[cfg(not(feature = "syntect"))]
#[derive(Copy, Clone)]
struct HighlightSettings<'a>(&'a ());

impl std::hash::Hash for HighlightSettings<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.0, state);
    }
}

#[derive(Default)]
struct Highlighter;

impl Highlighter {
    fn highlight(
        font_id: egui::FontId,
        theme: &CodeTheme,
        code: &str,
        lang: &str,
        settings: HighlightSettings<'_>,
    ) -> LayoutJob {
        Self::highlight_impl(theme, code, lang, settings).unwrap_or_else(|| {
            // Fallback:
            LayoutJob::simple(
                code.into(),
                font_id,
                if theme.dark_mode {
                    egui::Color32::LIGHT_GRAY
                } else {
                    egui::Color32::DARK_GRAY
                },
                f32::INFINITY,
            )
        })
    }

    #[cfg(feature = "syntect")]
    fn highlight_impl(
        theme: &CodeTheme,
        text: &str,
        language: &str,
        highlighter: HighlightSettings<'_>,
    ) -> Option<LayoutJob> {
        profiling::function_scope!();
        use syntect::easy::HighlightLines;
        use syntect::highlighting::FontStyle;
        use syntect::util::LinesWithEndings;

        let syntax = highlighter
            .0
            .ps
            .find_syntax_by_name(language)
            .or_else(|| highlighter.0.ps.find_syntax_by_extension(language))?;

        let syn_theme = theme.syntect_theme.syntect_key_name();
        let mut h = HighlightLines::new(syntax, &highlighter.0.ts.themes[syn_theme]);

        use egui::text::{LayoutSection, TextFormat};

        let mut job = LayoutJob {
            text: text.into(),
            ..Default::default()
        };

        for line in LinesWithEndings::from(text) {
            for (style, range) in h.highlight_line(line, &highlighter.0.ps).ok()? {
                let fg = style.foreground;
                let text_color = egui::Color32::from_rgb(fg.r, fg.g, fg.b);
                let italics = style.font_style.contains(FontStyle::ITALIC);
                let underline = style.font_style.contains(FontStyle::ITALIC);
                let underline = if underline {
                    egui::Stroke::new(1.0, text_color)
                } else {
                    egui::Stroke::NONE
                };
                job.sections.push(LayoutSection {
                    leading_space: 0.0,
                    byte_range: as_byte_range(text, range),
                    format: TextFormat {
                        font_id: theme.font_id.clone(),
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
    assert!(
        whole_start <= range_start,
        "range must be within whole, but was {range}"
    );
    assert!(
        range_start + range.len() <= whole_start + whole.len(),
        "range_start + range length must be smaller than whole_start + whole length, but was {}",
        range_start + range.len()
    );
    let offset = range_start - whole_start;
    offset..(offset + range.len())
}

// ----------------------------------------------------------------------------

#[cfg(not(feature = "syntect"))]
impl Highlighter {
    fn highlight_impl(
        theme: &CodeTheme,
        mut text: &str,
        language: &str,
        _settings: HighlightSettings<'_>,
    ) -> Option<LayoutJob> {
        profiling::function_scope!();

        let language = Language::new(language)?;

        // Extremely simple syntax highlighter for when we compile without syntect

        let mut job = LayoutJob::default();

        while !text.is_empty() {
            if language.double_slash_comments && text.starts_with("//")
                || language.hash_comments && text.starts_with('#')
            {
                let end = text.find('\n').unwrap_or(text.len());
                job.append(&text[..end], 0.0, theme.formats[TokenType::Comment].clone());
                text = &text[end..];
            } else if text.starts_with('"') {
                let end = text[1..]
                    .find('"')
                    .map(|i| i + 2)
                    .or_else(|| text.find('\n'))
                    .unwrap_or(text.len());
                job.append(
                    &text[..end],
                    0.0,
                    theme.formats[TokenType::StringLiteral].clone(),
                );
                text = &text[end..];
            } else if text.starts_with(|c: char| c.is_ascii_alphanumeric()) {
                let end = text[1..]
                    .find(|c: char| !c.is_ascii_alphanumeric())
                    .map_or_else(|| text.len(), |i| i + 1);
                let word = &text[..end];
                let tt = if language.is_keyword(word) {
                    TokenType::Keyword
                } else {
                    TokenType::Literal
                };
                job.append(word, 0.0, theme.formats[tt].clone());
                text = &text[end..];
            } else if text.starts_with(|c: char| c.is_ascii_whitespace()) {
                let end = text[1..]
                    .find(|c: char| !c.is_ascii_whitespace())
                    .map_or_else(|| text.len(), |i| i + 1);
                job.append(
                    &text[..end],
                    0.0,
                    theme.formats[TokenType::Whitespace].clone(),
                );
                text = &text[end..];
            } else {
                let mut it = text.char_indices();
                it.next();
                let end = it.next().map_or(text.len(), |(idx, _chr)| idx);
                job.append(
                    &text[..end],
                    0.0,
                    theme.formats[TokenType::Punctuation].clone(),
                );
                text = &text[end..];
            }
        }

        Some(job)
    }
}

#[cfg(not(feature = "syntect"))]
struct Language {
    /// `// comment`
    double_slash_comments: bool,

    /// `# comment`
    hash_comments: bool,

    keywords: std::collections::BTreeSet<&'static str>,
}

#[cfg(not(feature = "syntect"))]
impl Language {
    fn new(language: &str) -> Option<Self> {
        match language.to_lowercase().as_str() {
            "c" | "h" | "hpp" | "cpp" | "c++" => Some(Self::cpp()),
            "py" | "python" => Some(Self::python()),
            "rs" | "rust" => Some(Self::rust()),
            "toml" => Some(Self::toml()),
            _ => {
                None // unsupported language
            }
        }
    }

    fn is_keyword(&self, word: &str) -> bool {
        self.keywords.contains(word)
    }

    fn cpp() -> Self {
        Self {
            double_slash_comments: true,
            hash_comments: false,
            keywords: [
                "alignas",
                "alignof",
                "and_eq",
                "and",
                "asm",
                "atomic_cancel",
                "atomic_commit",
                "atomic_noexcept",
                "auto",
                "bitand",
                "bitor",
                "bool",
                "break",
                "case",
                "catch",
                "char",
                "char16_t",
                "char32_t",
                "char8_t",
                "class",
                "co_await",
                "co_return",
                "co_yield",
                "compl",
                "concept",
                "const_cast",
                "const",
                "consteval",
                "constexpr",
                "constinit",
                "continue",
                "decltype",
                "default",
                "delete",
                "do",
                "double",
                "dynamic_cast",
                "else",
                "enum",
                "explicit",
                "export",
                "extern",
                "false",
                "float",
                "for",
                "friend",
                "goto",
                "if",
                "inline",
                "int",
                "long",
                "mutable",
                "namespace",
                "new",
                "noexcept",
                "not_eq",
                "not",
                "nullptr",
                "operator",
                "or_eq",
                "or",
                "private",
                "protected",
                "public",
                "reflexpr",
                "register",
                "reinterpret_cast",
                "requires",
                "return",
                "short",
                "signed",
                "sizeof",
                "static_assert",
                "static_cast",
                "static",
                "struct",
                "switch",
                "synchronized",
                "template",
                "this",
                "thread_local",
                "throw",
                "true",
                "try",
                "typedef",
                "typeid",
                "typename",
                "union",
                "unsigned",
                "using",
                "virtual",
                "void",
                "volatile",
                "wchar_t",
                "while",
                "xor_eq",
                "xor",
            ]
            .into_iter()
            .collect(),
        }
    }

    fn python() -> Self {
        Self {
            double_slash_comments: false,
            hash_comments: true,
            keywords: [
                "and", "as", "assert", "break", "class", "continue", "def", "del", "elif", "else",
                "except", "False", "finally", "for", "from", "global", "if", "import", "in", "is",
                "lambda", "None", "nonlocal", "not", "or", "pass", "raise", "return", "True",
                "try", "while", "with", "yield",
            ]
            .into_iter()
            .collect(),
        }
    }

    fn rust() -> Self {
        Self {
            double_slash_comments: true,
            hash_comments: false,
            keywords: [
                "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else",
                "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match",
                "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct",
                "super", "trait", "true", "type", "unsafe", "use", "where", "while",
            ]
            .into_iter()
            .collect(),
        }
    }

    fn toml() -> Self {
        Self {
            double_slash_comments: false,
            hash_comments: true,
            keywords: Default::default(),
        }
    }
}
