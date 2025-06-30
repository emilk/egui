use crate::{
    epaint, text_selection, AtomLayout, CursorIcon, IntoAtoms, Response, Sense, Stroke, Ui, Widget,
    WidgetInfo, WidgetText, WidgetType,
};

use self::text_selection::LabelSelectionState;

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
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Link<'a> {
    layout: AtomLayout<'a>,
}

impl<'a> Link<'a> {
    pub fn new(atoms: impl IntoAtoms<'a>) -> Self {
        Self {
            layout: AtomLayout::new(atoms).sense(Sense::click()),
        }
    }
}

impl Widget for Link<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self { layout } = self;

        let color = ui.visuals().hyperlink_color;
        let text = layout.atoms.text().map(String::from);
        let layout_with_color = layout.fallback_text_color(color);

        let allocated = layout_with_color.allocate(ui);
        let response = allocated.response.clone();

        response.widget_info(|| {
            WidgetInfo::labeled(
                WidgetType::Link,
                ui.is_enabled(),
                text.as_deref().unwrap_or(""),
            )
        });

        if ui.is_rect_visible(response.rect) {
            let visuals = ui.style().interact(&response);

            let underline = if response.hovered() || response.has_focus() {
                Stroke::new(visuals.fg_stroke.width, color)
            } else {
                Stroke::NONE
            };

            let selectable = ui.style().interaction.selectable_labels;

            for galley in allocated.iter_texts() {
                let galley_pos = response.rect.min;

                if selectable {
                    LabelSelectionState::label_text_selection(
                        ui,
                        &response,
                        galley_pos,
                        galley.clone(),
                        color,
                        underline,
                    );
                } else {
                    ui.painter().add(
                        epaint::TextShape::new(galley_pos, galley.clone(), color)
                            .with_underline(underline),
                    );
                }
            }

            if response.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
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
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Hyperlink {
    url: String,
    text: WidgetText,
    new_tab: bool,
}

impl Hyperlink {
    #[expect(clippy::needless_pass_by_value)]
    pub fn new(url: impl ToString) -> Self {
        let url = url.to_string();
        Self {
            url: url.clone(),
            text: url.into(),
            new_tab: false,
        }
    }

    #[expect(clippy::needless_pass_by_value)]
    pub fn from_label_and_url(text: impl Into<WidgetText>, url: impl ToString) -> Self {
        Self {
            url: url.to_string(),
            text: text.into(),
            new_tab: false,
        }
    }

    /// Always open this hyperlink in a new browser tab.
    #[inline]
    pub fn open_in_new_tab(mut self, new_tab: bool) -> Self {
        self.new_tab = new_tab;
        self
    }
}

impl Widget for Hyperlink {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self { url, text, new_tab } = self;

        let response = ui.add(Link::new(text));

        if response.clicked_with_open_in_background() {
            ui.ctx().open_url(crate::OpenUrl {
                url: url.clone(),
                new_tab: true,
            });
        } else if response.clicked() {
            ui.ctx().open_url(crate::OpenUrl {
                url: url.clone(),
                new_tab,
            });
        }

        if ui.style().url_in_tooltip {
            response.on_hover_text(url)
        } else {
            response
        }
    }
}
