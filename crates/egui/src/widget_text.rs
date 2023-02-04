use std::borrow::Cow;
use std::sync::Arc;

use crate::{
    style::WidgetVisuals, text::LayoutJob, Align, Color32, FontFamily, FontSelection, Galley, Pos2,
    Style, TextStyle, Ui, Visuals,
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
        RichText::new(text)
    }
}

impl From<&String> for RichText {
    #[inline]
    fn from(text: &String) -> Self {
        RichText::new(text)
    }
}

impl From<&mut String> for RichText {
    #[inline]
    fn from(text: &mut String) -> Self {
        RichText::new(text.clone())
    }
}

impl From<String> for RichText {
    #[inline]
    fn from(text: String) -> Self {
        RichText::new(text)
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

    fn into_text_job(
        self,
        style: &Style,
        fallback_font: FontSelection,
        default_valign: Align,
    ) -> WidgetTextJob {
        let text_color = self.get_text_color(&style.visuals);

        let Self {
            text,
            size,
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

        let job_has_color = text_color.is_some();
        let line_color = text_color.unwrap_or_else(|| style.visuals.text_color());
        let text_color = text_color.unwrap_or(crate::Color32::TEMPORARY_COLOR);

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

        let text_format = crate::text::TextFormat {
            font_id,
            color: text_color,
            background: background_color,
            italics,
            underline,
            strikethrough,
            valign,
        };

        let job = LayoutJob::single_section(text, text_format);
        WidgetTextJob { job, job_has_color }
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
/// or text that has already been layed out in a [`Galley`].
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
    LayoutJob(LayoutJob),

    /// Use exactly this galley when painting the text.
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

    pub fn into_text_job(
        self,
        style: &Style,
        fallback_font: FontSelection,
        default_valign: Align,
    ) -> WidgetTextJob {
        match self {
            Self::RichText(text) => text.into_text_job(style, fallback_font, default_valign),
            Self::LayoutJob(job) => WidgetTextJob {
                job,
                job_has_color: true,
            },
            Self::Galley(galley) => {
                let job: LayoutJob = (*galley.job).clone();
                WidgetTextJob {
                    job,
                    job_has_color: true,
                }
            }
        }
    }

    /// Layout with wrap mode based on the containing [`Ui`].
    ///
    /// wrap: override for [`Ui::wrap_text`].
    pub fn into_galley(
        self,
        ui: &Ui,
        wrap: Option<bool>,
        available_width: f32,
        fallback_font: impl Into<FontSelection>,
    ) -> WidgetTextGalley {
        let wrap = wrap.unwrap_or_else(|| ui.wrap_text());
        let wrap_width = if wrap { available_width } else { f32::INFINITY };

        match self {
            Self::RichText(text) => {
                let valign = ui.layout().vertical_align();
                let mut text_job = text.into_text_job(ui.style(), fallback_font.into(), valign);
                text_job.job.wrap.max_width = wrap_width;
                WidgetTextGalley {
                    galley: ui.fonts(|f| f.layout_job(text_job.job)),
                    galley_has_color: text_job.job_has_color,
                }
            }
            Self::LayoutJob(mut job) => {
                job.wrap.max_width = wrap_width;
                WidgetTextGalley {
                    galley: ui.fonts(|f| f.layout_job(job)),
                    galley_has_color: true,
                }
            }
            Self::Galley(galley) => WidgetTextGalley {
                galley,
                galley_has_color: true,
            },
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

// ----------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
pub struct WidgetTextJob {
    pub job: LayoutJob,
    pub job_has_color: bool,
}

impl WidgetTextJob {
    pub fn into_galley(self, fonts: &crate::text::Fonts) -> WidgetTextGalley {
        let Self { job, job_has_color } = self;
        let galley = fonts.layout_job(job);
        WidgetTextGalley {
            galley,
            galley_has_color: job_has_color,
        }
    }
}

// ----------------------------------------------------------------------------

/// Text that has been layed out and ready to be painted.
#[derive(Clone, PartialEq)]
pub struct WidgetTextGalley {
    pub galley: Arc<Galley>,
    pub galley_has_color: bool,
}

impl WidgetTextGalley {
    /// Size of the layed out text.
    #[inline]
    pub fn size(&self) -> crate::Vec2 {
        self.galley.size()
    }

    /// Size of the layed out text.
    #[inline]
    pub fn text(&self) -> &str {
        self.galley.text()
    }

    #[inline]
    pub fn galley(&self) -> &Arc<Galley> {
        &self.galley
    }

    /// Use the colors in the original [`WidgetText`] if any,
    /// else fall back to the one specified by the [`WidgetVisuals`].
    pub fn paint_with_visuals(
        self,
        painter: &crate::Painter,
        text_pos: Pos2,
        visuals: &WidgetVisuals,
    ) {
        self.paint_with_fallback_color(painter, text_pos, visuals.text_color());
    }

    /// Use the colors in the original [`WidgetText`] if any,
    /// else fall back to the given color.
    pub fn paint_with_fallback_color(
        self,
        painter: &crate::Painter,
        text_pos: Pos2,
        text_color: Color32,
    ) {
        if self.galley_has_color {
            painter.galley(text_pos, self.galley);
        } else {
            painter.galley_with_color(text_pos, self.galley, text_color);
        }
    }

    /// Paint with this specific color.
    pub fn paint_with_color_override(
        self,
        painter: &crate::Painter,
        text_pos: Pos2,
        text_color: Color32,
    ) {
        painter.galley_with_color(text_pos, self.galley, text_color);
    }
}
