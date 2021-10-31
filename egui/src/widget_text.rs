use std::sync::Arc;

use crate::{style::WidgetVisuals, text::LayoutJob, Color32, Galley, Pos2, TextStyle, Ui};

/// Text and optional style choices for it.
///
/// The style choices (font, color) are applied to the entire text.
/// For more detailed control, use [`crate::text::LayoutJob`] instead.
#[derive(Default)]
pub struct RichText {
    text: String,
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

impl RichText {
    #[inline]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Default::default()
        }
    }

    #[inline]
    pub fn text(&self) -> &str {
        &self.text
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
    pub fn font_height(&self, fonts: &epaint::text::Fonts, style: &crate::Style) -> f32 {
        let text_style = self
            .text_style
            .or(style.override_text_style)
            .unwrap_or(style.body_text_style);
        fonts.row_height(text_style)
    }

    fn layout_job(self, ui: &Ui, wrap_width: f32, default_text_style: TextStyle) -> WidgetTextJob {
        let text_color = self.get_text_color(ui);

        let Self {
            text,
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
        let line_color = text_color.unwrap_or_else(|| ui.visuals().text_color());
        let text_color = text_color.unwrap_or(crate::Color32::TEMPORARY_COLOR);

        let text_style = text_style
            .or(ui.style().override_text_style)
            .unwrap_or(default_text_style);

        let mut background_color = background_color;
        if code {
            background_color = ui.visuals().code_bg_color;
        }
        let underline = if underline {
            crate::Stroke::new(1.0, line_color)
        } else {
            crate::Stroke::none()
        };
        let strikethrough = if strikethrough {
            crate::Stroke::new(1.0, line_color)
        } else {
            crate::Stroke::none()
        };

        let valign = if raised {
            crate::Align::TOP
        } else {
            ui.layout().vertical_align()
        };

        let text_format = crate::text::TextFormat {
            style: text_style,
            color: text_color,
            background: background_color,
            italics,
            underline,
            strikethrough,
            valign,
        };

        let mut job = LayoutJob::single_section(text, text_format);
        job.wrap_width = wrap_width;
        WidgetTextJob { job, job_has_color }
    }

    fn get_text_color(&self, ui: &Ui) -> Option<Color32> {
        if let Some(text_color) = self.text_color {
            Some(text_color)
        } else if self.strong {
            Some(ui.visuals().strong_text_color())
        } else if self.weak {
            Some(ui.visuals().weak_text_color())
        } else {
            ui.visuals().override_text_color
        }
    }

    pub fn layout(
        self,
        ui: &Ui,
        wrap_width: f32,
        default_text_style: TextStyle,
    ) -> WidgetTextGalley {
        let job = self.layout_job(ui, wrap_width, default_text_style);
        job.layout(ui.fonts())
    }
}

// ----------------------------------------------------------------------------

/// This is how you specify text for a widget.
///
/// Often this is just a simple [`String`],
/// but it can be a [`RichText`] (text with color, style, etc),
/// a [`LayoutJob`] (for when you want full control of how the text looks)
/// or text that has already been layed out in a [`Galley`].
pub enum WidgetText {
    RichText(RichText),
    LayoutJob(LayoutJob),
    Galley(Arc<Galley>),
}

impl WidgetText {
    #[inline]
    pub fn text(&self) -> &str {
        match self {
            Self::RichText(text) => text.text(),
            Self::LayoutJob(job) => &job.text,
            Self::Galley(galley) => galley.text(),
        }
    }

    /// Override the [`TextStyle`] if, and only if, this is a [`RichText`].
    #[inline]
    pub fn text_style(self, text_style: TextStyle) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.text_style(text_style)),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    /// Set the [`TextStyle`] unless it has already been set
    #[inline]
    pub fn fallback_text_style(self, text_style: TextStyle) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.fallback_text_style(text_style)),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    /// Override text color if, and only if, this is a [`RichText`].
    #[inline]
    pub fn color(self, color: impl Into<Color32>) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.color(color)),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    pub fn font_height(&self, fonts: &epaint::text::Fonts, style: &crate::Style) -> f32 {
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

    /// wrap: override for [`Ui::wrap_text`].
    pub fn layout(
        self,
        ui: &Ui,
        wrap: Option<bool>,
        available_width: f32,
        default_text_style: TextStyle,
    ) -> WidgetTextGalley {
        let wrap = wrap.unwrap_or_else(|| ui.wrap_text());
        let wrap_width = if wrap { available_width } else { f32::INFINITY };

        match self {
            Self::RichText(text) => text.layout(ui, wrap_width, default_text_style),
            Self::LayoutJob(mut job) => {
                job.wrap_width = wrap_width;
                WidgetTextGalley {
                    galley: ui.fonts().layout_job(job),
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

impl From<String> for WidgetText {
    #[inline]
    fn from(text: String) -> Self {
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

pub struct WidgetTextJob {
    job: LayoutJob,
    job_has_color: bool,
}

impl WidgetTextJob {
    pub fn layout(self, fonts: &crate::text::Fonts) -> WidgetTextGalley {
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
pub struct WidgetTextGalley {
    galley: Arc<Galley>,
    galley_has_color: bool,
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

    pub fn paint(self, ui: &Ui, text_pos: Pos2, visuals: &WidgetVisuals) {
        if self.galley_has_color {
            ui.painter().galley(text_pos, self.galley);
        } else {
            ui.painter()
                .galley_with_color(text_pos, self.galley, visuals.text_color());
        }
    }

    pub fn paint_with_color(self, ui: &Ui, text_pos: Pos2, text_color: Color32) {
        ui.painter()
            .galley_with_color(text_pos, self.galley, text_color);
    }
}
