use crate::*;

/// Clickable text, that looks like a hyperlink.
///
/// To link to a web page, use [`Hyperlink`], [`Ui::hyperlink`] or [`Ui::hyperlink_to`].
///
/// See also [`Ui::link`].
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// // These are equivalent:
/// if ui.link("Documentation").clicked() {
///     // …
/// }
///
/// if ui.add(egui::Link::new("Documentation")).clicked() {
///     // …
/// }
/// # });
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Link {
    text: WidgetText,
}

impl Link {
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self { text: text.into() }
    }
}

impl Widget for Link {
    fn ui(self, ui: &mut Ui) -> Response {
        let Link { text } = self;
        let label = Label::new(text).sense(Sense::click());

        let (pos, text_galley, response) = label.layout_in_ui(ui);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Link, text_galley.text()));

        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }

        if ui.is_rect_visible(response.rect) {
            let color = ui.visuals().hyperlink_color;
            let visuals = ui.style().interact(&response);

            let underline = if response.hovered() || response.has_focus() {
                Stroke::new(visuals.fg_stroke.width, color)
            } else {
                Stroke::NONE
            };

            ui.painter().add(epaint::TextShape {
                pos,
                galley: text_galley.galley,
                override_text_color: Some(color),
                underline,
                angle: 0.0,
            });
        }

        response
    }
}

/// A clickable hyperlink, e.g. to `"https://github.com/emilk/egui"`.
///
/// See also [`Ui::hyperlink`] and [`Ui::hyperlink_to`].
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// // These are equivalent:
/// ui.hyperlink("https://github.com/emilk/egui");
/// ui.add(egui::Hyperlink::new("https://github.com/emilk/egui"));
///
/// // These are equivalent:
/// ui.hyperlink_to("My favorite repo", "https://github.com/emilk/egui");
/// ui.add(egui::Hyperlink::from_label_and_url("My favorite repo", "https://github.com/emilk/egui"));
/// # });
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Hyperlink {
    url: String,
    text: WidgetText,
}

impl Hyperlink {
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(url: impl ToString) -> Self {
        let url = url.to_string();
        Self {
            url: url.clone(),
            text: url.into(),
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn from_label_and_url(text: impl Into<WidgetText>, url: impl ToString) -> Self {
        Self {
            url: url.to_string(),
            text: text.into(),
        }
    }
}

impl Widget for Hyperlink {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self { url, text } = self;

        let response = ui.add(Link::new(text));
        if response.clicked() {
            let modifiers = ui.ctx().input(|i| i.modifiers);
            ui.ctx().output_mut(|o| {
                o.open_url = Some(crate::output::OpenUrl {
                    url: url.clone(),
                    new_tab: modifiers.any(),
                });
            });
        }
        if response.middle_clicked() {
            ui.ctx().output_mut(|o| {
                o.open_url = Some(crate::output::OpenUrl {
                    url: url.clone(),
                    new_tab: true,
                });
            });
        }
        response.on_hover_text(url)
    }
}
