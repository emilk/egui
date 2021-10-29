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
    text_color: Option<Color32>,
    wrap: Option<bool>,
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

    /// Override text color.
    #[inline]
    pub fn color(mut self, color: impl Into<Color32>) -> Self {
        self.text_color = Some(color.into());
        self
    }

    /// If `true`, the text will wrap at the `max_width`.
    ///
    /// By default [`Self::wrap`] will be true in vertical layouts
    /// and horizontal layouts with wrapping,
    /// and false on non-wrapping horizontal layouts.
    ///
    /// Note that any `\n` in the text will always produce a new line.
    #[inline]
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = Some(wrap);
        self
    }

    pub fn layout(
        self,
        ui: &Ui,
        wrap_width: f32,
        default_text_style: TextStyle,
    ) -> WidgetTextLayout {
        let Self {
            text,
            text_style,
            text_color,
            wrap,
        } = self;

        let wrap = wrap.unwrap_or_else(|| ui.wrap_text());
        let wrap_width = if wrap { wrap_width } else { f32::INFINITY };

        let text_style = text_style
            .or(ui.style().override_text_style)
            .unwrap_or(default_text_style);

        let text_color = text_color.or(ui.visuals().override_text_color);

        if let Some(text_color) = text_color {
            let galley = ui.fonts().layout(text, text_style, text_color, wrap_width);

            WidgetTextLayout {
                galley,
                galley_has_color: true,
            }
        } else {
            let galley = ui
                .fonts()
                .layout_delayed_color(text, text_style, wrap_width);

            WidgetTextLayout {
                galley,
                galley_has_color: false,
            }
        }
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

    /// Override text color if, and only if, this is a [`RichText`].
    #[inline]
    pub fn color(self, color: impl Into<Color32>) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.color(color)),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    /// Override text wrapping behavior if, and only if, this is a [`RichText`].
    #[inline]
    pub fn wrap(self, wrap: bool) -> Self {
        match self {
            Self::RichText(text) => Self::RichText(text.wrap(wrap)),
            Self::LayoutJob(_) | Self::Galley(_) => self,
        }
    }

    pub fn layout(
        self,
        ui: &Ui,
        wrap_width: f32,
        default_text_style: TextStyle,
    ) -> WidgetTextLayout {
        match self {
            Self::RichText(text) => text.layout(ui, wrap_width, default_text_style),
            Self::LayoutJob(mut job) => {
                job.wrap_width = wrap_width;
                WidgetTextLayout {
                    galley: ui.fonts().layout_job(job),
                    galley_has_color: true,
                }
            }
            Self::Galley(galley) => WidgetTextLayout {
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

/// Text that has been layed out and ready to be painted.
pub struct WidgetTextLayout {
    galley: Arc<Galley>,
    galley_has_color: bool,
}

impl WidgetTextLayout {
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

    pub fn paint(self, ui: &Ui, text_pos: Pos2, visuals: &WidgetVisuals) {
        if self.galley_has_color {
            ui.painter().galley(text_pos, self.galley);
        } else {
            ui.painter()
                .galley_with_color(text_pos, self.galley, visuals.text_color());
        }
    }
}
