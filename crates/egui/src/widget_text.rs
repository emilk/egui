use std::{borrow::Cow, sync::Arc};

use crate::{
    text::{LayoutJob, TextWrapping},
    Align, Color32, FontFamily, FontSelection, Galley, Style, TextStyle, TextWrapMode, Ui, Visuals,
};

/// Text and optional style choices for it.
///
/// The style choices (font, color) are applied to the entire text.
/// For more detailed control, use [`crate::text::LayoutJob`] instead.
///
/// A [`RichText`] can be used in most widgets and helper functions, e.g. [`Ui::label`] and [`Ui::button`].
///
/// ### Example
/// ```
/// use egui::{RichText, Color32};
///
/// RichText::new("Plain");
/// RichText::new("colored").color(Color32::RED);
/// RichText::new("Large and underlined").size(20.0).underline();
/// ```
#[derive(Clone, Default, PartialEq)]
pub struct RichText {
    text: String,
    size: Option<f32>,
    extra_letter_spacing: f32,
    line_height: Option<f32>,
    family: Option<FontFamily>,
    text_style: Option<TextStyle>,
    background_color: Color32,
    text_color: Option<Color32>,
    code: bool,
    strong: bool,
    weak: bool,
    strikethrough: bool,
    underline: bool,
    italics: bool,
    raised: bool,
}

impl From<&str> for RichText {
    #[inline]
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<&String> for RichText {
    #[inline]
    fn from(text: &String) -> Self {
        Self::new(text)
    }
}

impl From<&mut String> for RichText {
    #[inline]
    fn from(text: &mut String) -> Self {
        Self::new(text.clone())
    }
}

impl From<String> for RichText {
    #[inline]
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

impl From<Cow<'_, str>> for RichText {
    #[inline]
    fn from(text: Cow<'_, str>) -> Self {
        Self::new(text)
    }
}

impl RichText {
    #[inline]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Default::default()
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    #[inline]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Select the font size (in points).
    /// This overrides the value from [`Self::text_style`].
    #[inline]
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// Extra spacing between letters, in points.
    ///
    /// Default: 0.0.
    ///
    /// For even text it is recommended you round this to an even number of _pixels_,
    /// e.g. using [`crate::Painter::round_to_pixel`].
    #[inline]
    pub fn extra_letter_spacing(mut self, extra_letter_spacing: f32) -> Self {
        self.extra_letter_spacing = extra_letter_spacing;
        self
    }

    /// Explicit line height of the text in points.
    ///
    /// This is the distance between the bottom row of two subsequent lines of text.
    ///
    /// If `None` (the default), the line height is determined by the font.
    ///
    /// For even text it is recommended you round this to an even number of _pixels_,
    /// e.g. using [`crate::Painter::round_to_pixel`].
    #[inline]
    pub fn line_height(mut self, line_height: Option<f32>) -> Self {
        self.line_height = line_height;
        self
    }

    /// Select the font family.
    ///
    /// This overrides the value from [`Self::text_style`].
    ///
    /// Only the families available in [`crate::FontDefinitions::families`] may be used.
    #[inline]
    pub fn family(mut self, family: FontFamily) -> Self {
        self.family = Some(family);
        self
    }

    /// Select the font and size.
    /// This overrides the value from [`Self::text_style`].
    #[inline]
    pub fn font(mut self, font_id: crate::FontId) -> Self {
        let crate::FontId { size, family } = font_id;
        self.size = Some(size);
        self.family = Some(family);
        self
    }

    /// Override the [`TextStyle`].
    #[inline]
    pub fn text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = Some(text_style);
        self
    }

    /// Set the [`TextStyle`] unless it has already been set
    #[inline]
    pub fn fallback_text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style.get_or_insert(text_style);
        self
    }

    /// Use [`TextStyle::Heading`].
    #[inline]
    pub fn heading(self) -> Self {
        self.text_style(TextStyle::Heading)
    }

    /// Use [`TextStyle::Monospace`].
    #[inline]
    pub fn monospace(self) -> Self {
        self.text_style(TextStyle::Monospace)
    }

    /// Monospace label with different background color.
    #[inline]
    pub fn code(mut self) -> Self {
        self.code = true;
        self.text_style(TextStyle::Monospace)
    }

    /// Extra strong text (stronger color).
    #[inline]
    pub fn strong(mut self) -> Self {
        self.strong = true;
        self
    }

    /// Extra weak text (fainter color).
    #[inline]
    pub fn weak(mut self) -> Self {
        self.weak = true;
        self
    }

    /// Draw a line under the text.
    ///
    /// If you want to control the line color, use [`LayoutJob`] instead.
    #[inline]
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    /// Draw a line through the text, crossing it out.
    ///
    /// If you want to control the strikethrough line color, use [`LayoutJob`] instead.
    #[inline]
    pub fn strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }

    /// Tilt the characters to the right.
    #[inline]
    pub fn italics(mut self) -> Self {
        self.italics = true;
        self
    }

    /// Smaller text.
    #[inline]
    pub fn small(self) -> Self {
        self.text_style(TextStyle::Small)
    }

    /// For e.g. exponents.
    #[inline]
    pub fn small_raised(self) -> Self {
        self.text_style(TextStyle::Small).raised()
    }

    /// Align text to top. Only applicable together with [`Self::small()`].
    #[inline]
    pub fn raised(mut self) -> Self {
        self.raised = true;
        self
    }

    /// Fill-color behind the text.
    #[inline]
    pub fn background_color(mut self, background_color: impl Into<Color32>) -> Self {
        self.background_color = background_color.into();
        self
    }

    /// Override text color.
    ///
    /// If not set, [`Color32::PLACEHOLDER`] will be used,
    /// which will be replaced with a color chosen by the widget that paints the text.
    #[inline]
    pub fn color(mut self, color: impl Into<Color32>) -> Self {
        self.text_color = Some(color.into());
        self
    }

    /// Read the font height of the selected text style.
    pub fn font_height(&self, fonts: &epaint::Fonts, style: &Style) -> f32 {
        let mut font_id = self.text_style.as_ref().map_or_else(
            || FontSelection::Default.resolve(style),
            |text_style| text_style.resolve(style),
        );

        if let Some(size) = self.size {
            font_id.size = size;
        }
        if let Some(family) = &self.family {
            font_id.family = family.clone();
        }
        fonts.row_height(&font_id)
    }

    /// Append to an existing [`LayoutJob`]
    ///
    /// Note that the color of the [`RichText`] must be set, or may default to an undesirable color.
    ///
    /// ### Example
    /// ```
    /// use egui::{Style, RichText, text::LayoutJob, Color32, FontSelection, Align};
    ///
    /// let style = Style::default();
    /// let mut layout_job = LayoutJob::default();
    /// RichText::new("Normal")
    ///     .color(style.visuals.text_color())
    ///     .append_to(
    ///         &mut layout_job,
    ///         &style,
    ///         FontSelection::Default,
    ///         Align::Center,
    ///     );
    /// RichText::new("Large and underlined")
    ///     .color(style.visuals.text_color())
    ///     .size(20.0)
    ///     .underline()
    ///     .append_to(
    ///         &mut layout_job,
    ///         &style,
    ///         FontSelection::Default,
    ///         Align::Center,
    ///     );
    /// ```
    pub fn append_to(
        self,
        layout_job: &mut LayoutJob,
        style: &Style,
        fallback_font: FontSelection,
        default_valign: Align,
    ) {
        let (text, format) = self.into_text_and_format(style, fallback_font, default_valign);

        layout_job.append(&text, 0.0, format);
    }

    fn into_layout_job(
        self,
        style: &Style,
        fallback_font: FontSelection,
        default_valign: Align,
    ) -> LayoutJob {
        let (text, text_format) = self.into_text_and_format(style, fallback_font, default_valign);
        LayoutJob::single_section(text, text_format)
    }

    fn into_text_and_format(
        self,
        style: &Style,
        fallback_font: FontSelection,
        default_valign: Align,
    ) -> (String, crate::text::TextFormat) {
        let text_color = self.get_text_color(&style.visuals);

        let Self {
            text,
            size,
            extra_letter_spacing,
            line_height,
            family,
            text_style,
            background_color,
            text_color: _, // already used by `get_text_color`
            code,
            strong: _, // already used by `get_text_color`
            weak: _,   // already used by `get_text_color`
            strikethrough,
            underline,
            italics,
            raised,
        } = self;

        let line_color = text_color.unwrap_or_else(|| style.visuals.text_color());
        let text_color = text_color.unwrap_or(crate::Color32::PLACEHOLDER);

        let font_id = {
            let mut font_id = text_style
                .or_else(|| style.override_text_style.clone())
                .map_or_else(
                    || fallback_font.resolve(style),
                    |text_style| text_style.resolve(style),
                );
            if let Some(size) = size {
                font_id.size = size;
            }
            if let Some(family) = family {
                font_id.family = family;
            }
            font_id
        };

        let mut background_color = background_color;
        if code {
            background_color = style.visuals.code_bg_color;
        }
        let underline = if underline {
            crate::Stroke::new(1.0, line_color)
        } else {
            crate::Stroke::NONE
        };
        let strikethrough = if strikethrough {
            crate::Stroke::new(1.0, line_color)
        } else {
            crate::Stroke::NONE
        };

        let valign = if raised {
            crate::Align::TOP
        } else {
            default_valign
        };

        (
            text,
            crate::text::TextFormat {
                font_id,
                extra_letter_spacing,
                line_height,
                color: text_color,
                background: background_color,
                italics,
                underline,
                strikethrough,
                valign,
            },
        )
    }

    fn get_text_color(&self, visuals: &Visuals) -> Option<Color32> {
        if let Some(text_color) = self.text_color {
            Some(text_color)
        } else if self.strong {
            Some(visuals.strong_text_color())
        } else if self.weak {
            Some(visuals.weak_text_color())
        } else {
            visuals.override_text_color
        }
    }
}

// ----------------------------------------------------------------------------

/// This is how you specify text for a widget.
///
/// A lot of widgets use `impl Into<WidgetText>` as an argument,
/// allowing you to pass in [`String`], [`RichText`], [`LayoutJob`], and more.
///
/// Often a [`WidgetText`] is just a simple [`String`],
/// but it can be a [`RichText`] (text with color, style, etc),
/// a [`LayoutJob`] (for when you want full control of how the text looks)
/// or text that has already been laid out in a [`Galley`].
///
/// You can color the text however you want, or use [`Color32::PLACEHOLDER`]
/// which will be replaced with a color chosen by the widget that paints the text.
#[derive(Clone)]
pub enum WidgetText {
    RichText(RichText),

    /// Use this [`LayoutJob`] when laying out the text.
    ///
    /// Only [`LayoutJob::text`] and [`LayoutJob::sections`] are guaranteed to be respected.
    ///
    /// [`TextWrapping::max_width`](epaint::text::TextWrapping::max_width), [`LayoutJob::halign`], [`LayoutJob::justify`]
    /// and [`LayoutJob::first_row_min_height`] will likely be determined by the [`crate::Layout`]
    /// of the [`Ui`] the widget is placed in.
    /// If you want all parts of the [`LayoutJob`] respected, then convert it to a
    /// [`Galley`] and use [`Self::Galley`] instead.
    ///
    /// You can color the text however you want, or use [`Color32::PLACEHOLDER`]
    /// which will be replaced with a color chosen by the widget that paints the text.
    LayoutJob(LayoutJob),

    /// Use exactly this galley when painting the text.
    ///
    /// You can color the text however you want, or use [`Color32::PLACEHOLDER`]
    /// which will be replaced with a color chosen by the widget that paints the text.
    Galley(Arc<Galley>),
}

impl Default for WidgetText {
    fn default() -> Self {
        Self::RichText(RichText::default())
    }
}

impl WidgetText {
    #[inline]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::RichText(text) => text.is_empty(),
            Self::LayoutJob(job) => job.is_empty(),
            Self::Galley(galley) => galley.is_empty(),
        }
    }

    #[inline]
    pub fn text(&self) -> &str {
        match self {
            Self::RichText(text) => text.text(),
            Self::LayoutJob(job) => &job.text,
            Self::Galley(galley) => galley.text(),
        }
    }

    /// Override the [`TextStyle`] if, and only if, this is a [`RichText`].
    ///
    /// Prefer using [`RichText`] directly!
    #[inline]
    pub fn text_style(self, text_style: TextStyle) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.text_style(text_style)),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    /// Set the [`TextStyle`] unless it has already been set
    ///
    /// Prefer using [`RichText`] directly!
    #[inline]
    pub fn fallback_text_style(self, text_style: TextStyle) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.fallback_text_style(text_style)),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    /// Override text color if, and only if, this is a [`RichText`].
    ///
    /// Prefer using [`RichText`] directly!
    #[inline]
    pub fn color(self, color: impl Into<Color32>) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.color(color)),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    /// Prefer using [`RichText`] directly!
    pub fn heading(self) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.heading()),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    /// Prefer using [`RichText`] directly!
    pub fn monospace(self) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.monospace()),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    /// Prefer using [`RichText`] directly!
    pub fn code(self) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.code()),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    /// Prefer using [`RichText`] directly!
    pub fn strong(self) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.strong()),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    /// Prefer using [`RichText`] directly!
    pub fn weak(self) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.weak()),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    /// Prefer using [`RichText`] directly!
    pub fn underline(self) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.underline()),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    /// Prefer using [`RichText`] directly!
    pub fn strikethrough(self) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.strikethrough()),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    /// Prefer using [`RichText`] directly!
    pub fn italics(self) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.italics()),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    /// Prefer using [`RichText`] directly!
    pub fn small(self) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.small()),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    /// Prefer using [`RichText`] directly!
    pub fn small_raised(self) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.small_raised()),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    /// Prefer using [`RichText`] directly!
    pub fn raised(self) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.raised()),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    /// Prefer using [`RichText`] directly!
    pub fn background_color(self, background_color: impl Into<Color32>) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.background_color(background_color)),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    pub(crate) fn font_height(&self, fonts: &epaint::Fonts, style: &Style) -> f32 {
        match self {
            Self::RichText(text) => text.font_height(fonts, style),
            Self::LayoutJob(job) => job.font_height(fonts),
            Self::Galley(galley) => {
                if let Some(row) = galley.rows.first() {
                    row.height()
                } else {
                    galley.size().y
                }
            }
        }
    }

    pub fn into_layout_job(
        self,
        style: &Style,
        fallback_font: FontSelection,
        default_valign: Align,
    ) -> LayoutJob {
        match self {
            Self::RichText(text) => text.into_layout_job(style, fallback_font, default_valign),
            Self::LayoutJob(job) => job,
            Self::Galley(galley) => (*galley.job).clone(),
        }
    }

    /// Layout with wrap mode based on the containing [`Ui`].
    ///
    /// `wrap_mode`: override for [`Ui::wrap_mode`]
    pub fn into_galley(
        self,
        ui: &Ui,
        wrap_mode: Option<TextWrapMode>,
        available_width: f32,
        fallback_font: impl Into<FontSelection>,
    ) -> Arc<Galley> {
        let valign = ui.layout().vertical_align();
        let style = ui.style();

        let wrap_mode = wrap_mode.unwrap_or_else(|| ui.wrap_mode());
        let text_wrapping = TextWrapping::from_wrap_mode_and_width(wrap_mode, available_width);

        self.into_galley_impl(ui.ctx(), style, text_wrapping, fallback_font.into(), valign)
    }

    pub fn into_galley_impl(
        self,
        ctx: &crate::Context,
        style: &Style,
        text_wrapping: TextWrapping,
        fallback_font: FontSelection,
        default_valign: Align,
    ) -> Arc<Galley> {
        match self {
            Self::RichText(text) => {
                let mut layout_job = text.into_layout_job(style, fallback_font, default_valign);
                layout_job.wrap = text_wrapping;
                ctx.fonts(|f| f.layout_job(layout_job))
            }
            Self::LayoutJob(mut job) => {
                job.wrap = text_wrapping;
                ctx.fonts(|f| f.layout_job(job))
            }
            Self::Galley(galley) => galley,
        }
    }
}

impl From<&str> for WidgetText {
    #[inline]
    fn from(text: &str) -> Self {
        Self::RichText(RichText::new(text))
    }
}

impl From<&String> for WidgetText {
    #[inline]
    fn from(text: &String) -> Self {
        Self::RichText(RichText::new(text))
    }
}

impl From<String> for WidgetText {
    #[inline]
    fn from(text: String) -> Self {
        Self::RichText(RichText::new(text))
    }
}

impl From<Cow<'_, str>> for WidgetText {
    #[inline]
    fn from(text: Cow<'_, str>) -> Self {
        Self::RichText(RichText::new(text))
    }
}

impl From<RichText> for WidgetText {
    #[inline]
    fn from(rich_text: RichText) -> Self {
        Self::RichText(rich_text)
    }
}

impl From<LayoutJob> for WidgetText {
    #[inline]
    fn from(layout_job: LayoutJob) -> Self {
        Self::LayoutJob(layout_job)
    }
}

impl From<Arc<Galley>> for WidgetText {
    #[inline]
    fn from(galley: Arc<Galley>) -> Self {
        Self::Galley(galley)
    }
}
