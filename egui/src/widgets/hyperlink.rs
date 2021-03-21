use crate::*;

/// A clickable hyperlink, e.g. to `"https://github.com/emilk/egui"`.
///
/// See also [`Ui::hyperlink`] and [`Ui::hyperlink_to`].
///
/// ```
/// # let ui = &mut egui::Ui::__test();
/// ui.hyperlink("https://github.com/emilk/egui");
/// ui.add(egui::Hyperlink::new("https://github.com/emilk/egui").text("My favorite repo").small());
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Hyperlink {
    url: String,
    label: Label,
}

impl Hyperlink {
    pub fn new(url: impl Into<String>) -> Self {
        let url = url.into();
        Self {
            url: url.clone(),
            label: Label::new(url),
        }
    }

    pub fn from_label_and_url(label: impl Into<Label>, url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            label: label.into(),
        }
    }

    /// Show some other text than the url
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.label.text = text.into();
        self
    }

    /// The default is [`Style::body_text_style`] (generally [`TextStyle::Body`]).
    pub fn text_style(mut self, text_style: TextStyle) -> Self {
        self.label = self.label.text_style(text_style);
        self
    }

    pub fn small(self) -> Self {
        self.text_style(TextStyle::Small)
    }
}

impl Widget for Hyperlink {
    fn ui(self, ui: &mut Ui) -> Response {
        let Hyperlink { url, label } = self;
        let galley = label.layout(ui);
        let (rect, response) = ui.allocate_exact_size(galley.size, Sense::click());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Hyperlink, &galley.text));

        if response.hovered() {
            ui.ctx().output().cursor_icon = CursorIcon::PointingHand;
        }
        if response.clicked() {
            let modifiers = ui.ctx().input().modifiers;
            ui.ctx().output().open_url = Some(crate::output::OpenUrl {
                url: url.clone(),
                new_tab: modifiers.any(),
            });
        }
        if response.middle_clicked() {
            ui.ctx().output().open_url = Some(crate::output::OpenUrl {
                url: url.clone(),
                new_tab: true,
            });
        }

        let color = ui.visuals().hyperlink_color;
        let visuals = ui.style().interact(&response);

        if response.hovered() || response.has_focus() {
            // Underline:
            for row in &galley.rows {
                let rect = row.rect().translate(rect.min.to_vec2());
                ui.painter().line_segment(
                    [rect.left_bottom(), rect.right_bottom()],
                    (visuals.fg_stroke.width, color),
                );
            }
        }

        let label = label.text_color(color);
        label.paint_galley(ui, rect.min, galley);

        response.on_hover_text(url)
    }
}
