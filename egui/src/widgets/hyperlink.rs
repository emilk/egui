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
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(url: impl ToString) -> Self {
        let url = url.to_string();
        Self {
            url: url.clone(),
            label: Label::new(url).sense(Sense::click()),
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn from_label_and_url(label: impl Into<Label>, url: impl ToString) -> Self {
        Self {
            url: url.to_string(),
            label: label.into(),
        }
    }

    /// Show some other text than the url
    #[allow(clippy::needless_pass_by_value)]
    pub fn text(mut self, text: impl ToString) -> Self {
        self.label.text = text.to_string();
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
        let (pos, galley, response) = label.layout_in_ui(ui);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Hyperlink, galley.text()));

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

        let underline = if response.hovered() || response.has_focus() {
            Stroke::new(visuals.fg_stroke.width, color)
        } else {
            Stroke::none()
        };

        ui.painter().add(Shape::Text {
            pos,
            galley,
            override_text_color: Some(color),
            underline,
        });

        response.on_hover_text(url)
    }
}
