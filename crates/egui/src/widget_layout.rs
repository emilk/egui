use crate::{Frame, ImageSource, Response, Sense, Ui, WidgetText};
use emath::Vec2;
use epaint::Galley;

enum WidgetLayoutItem<'a> {
    Text(WidgetText),
    Image(ImageSource<'a>),
    Custom(Vec2),
    Grow,
}

enum SizedWidgetLayoutItem<'a> {
    Text(Galley),
    Image(ImageSource<'a>, Vec2),
    Custom(Vec2),
    Grow,
}

struct WidgetLayout<'a> {
    items: Vec<WidgetLayoutItem<'a>>,
    gap: f32,
    frame: Frame,
    sense: Sense,
}

impl<'a> WidgetLayout<'a> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            gap: 0.0,
            frame: Frame::default(),
            sense: Sense::hover(),
        }
    }

    pub fn add(mut self, item: impl Into<WidgetLayoutItem<'a>>) -> Self {
        self.items.push(item.into());
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = frame;
        self
    }

    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let available_width = ui.available_width();

        let mut desired_width = 0.0;
        let mut preferred_width = 0.0;

        let mut height = 0.0;

        let mut sized_items = Vec::new();

        let (rect, response) = ui.allocate_at_least(Vec2::new(desired_width, height), self.sense);

        response
    }
}

struct WLButton<'a> {
    wl: WidgetLayout<'a>,
}

impl<'a> WLButton<'a> {
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            wl: WidgetLayout::new().add(text),
        }
    }

    pub fn ui(mut self, ui: &mut Ui) -> Response {
        let response = ui.ctx().read_response(ui.next_auto_id());

        let visuals = response.map_or(&ui.style().visuals.widgets.inactive, |response| {
            ui.style().interact(&response)
        });

        self.wl.frame = self
            .wl
            .frame
            .fill(visuals.bg_fill)
            .stroke(visuals.bg_stroke)
            .corner_radius(visuals.corner_radius);

        self.wl.show(ui)
    }
}
