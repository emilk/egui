use crate::{Frame, Image, ImageSource, Response, Sense, TextStyle, Ui, Widget, WidgetText};
use emath::{Align2, Vec2};
use epaint::Galley;
use std::sync::Arc;

enum WidgetLayoutItemType<'a> {
    Text(WidgetText),
    Image(Image<'a>),
    Custom(Vec2),
    Grow,
}

enum SizedWidgetLayoutItemType<'a> {
    Text(Arc<Galley>),
    Image(Image<'a>, Vec2),
    Custom(Vec2),
    Grow,
}

struct Item {
    align2: Align2,
}

impl Default for Item {
    fn default() -> Self {
        Self {
            align2: Align2::LEFT_CENTER,
        }
    }
}

impl SizedWidgetLayoutItemType<'_> {
    pub fn size(&self) -> Vec2 {
        match self {
            SizedWidgetLayoutItemType::Text(galley) => galley.size(),
            SizedWidgetLayoutItemType::Image(_, size) => *size,
            SizedWidgetLayoutItemType::Custom(size) => *size,
            SizedWidgetLayoutItemType::Grow => Vec2::ZERO,
        }
    }
}

struct WidgetLayout<'a> {
    items: Vec<(Item, WidgetLayoutItemType<'a>)>,
    gap: f32,
    frame: Frame,
    sense: Sense,
}

impl<'a> WidgetLayout<'a> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            gap: 4.0,
            frame: Frame::default(),
            sense: Sense::hover(),
        }
    }

    pub fn add(mut self, item: Item, kind: impl Into<WidgetLayoutItemType<'a>>) -> Self {
        self.items.push((item, kind.into()));
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
        let available_size = ui.available_size();
        let available_width = available_size.x;

        let mut desired_width = 0.0;
        let mut preferred_width = 0.0;

        let mut height: f32 = 0.0;

        let mut sized_items = Vec::new();

        let mut grow_count = 0;

        for (item, kind) in self.items {
            let (preferred_size, sized) = match kind {
                WidgetLayoutItemType::Text(text) => {
                    let galley = text.into_galley(ui, None, available_width, TextStyle::Button);
                    (
                        galley.size(), // TODO
                        SizedWidgetLayoutItemType::Text(galley),
                    )
                }
                WidgetLayoutItemType::Image(image) => {
                    let size =
                        image.load_and_calc_size(ui, Vec2::min(available_size, Vec2::splat(16.0)));
                    let size = size.unwrap_or_default();
                    (size, SizedWidgetLayoutItemType::Image(image, size))
                }
                WidgetLayoutItemType::Custom(size) => {
                    (size, SizedWidgetLayoutItemType::Custom(size))
                }
                WidgetLayoutItemType::Grow => {
                    grow_count += 1;
                    (Vec2::ZERO, SizedWidgetLayoutItemType::Grow)
                }
            };
            let size = sized.size();

            desired_width += size.x;
            preferred_width += preferred_size.x;

            height = height.max(size.y);

            sized_items.push((item, sized));
        }

        if sized_items.len() > 1 {
            let gap_space = self.gap * (sized_items.len() as f32 - 1.0);
            desired_width += gap_space;
            preferred_width += gap_space;
        }

        let margin = self.frame.total_margin();
        let content_size = Vec2::new(desired_width, height);
        let frame_size = content_size + margin.sum();

        let (rect, response) = ui.allocate_at_least(frame_size, self.sense);

        let content_rect = rect - margin;
        ui.painter().add(self.frame.paint(content_rect));

        let width_to_fill = content_rect.width();
        let extra_space = f32::max(width_to_fill - desired_width, 0.0);
        let grow_width = f32::max(extra_space / grow_count as f32, 0.0);

        let mut cursor = content_rect.left();

        for (item, sized) in sized_items {
            let size = sized.size();
            let width = match sized {
                SizedWidgetLayoutItemType::Grow => grow_width,
                _ => size.x,
            };

            let frame = content_rect.with_min_x(cursor).with_max_x(cursor + width);
            cursor = frame.right() + self.gap;

            let rect = item.align2.align_size_within_rect(size, frame);

            match sized {
                SizedWidgetLayoutItemType::Text(galley) => {
                    ui.painter()
                        .galley(rect.min, galley, ui.visuals().text_color());
                }
                SizedWidgetLayoutItemType::Image(image, _) => {
                    image.paint_at(ui, rect);
                }
                SizedWidgetLayoutItemType::Custom(_) => {}
                SizedWidgetLayoutItemType::Grow => {}
            }
        }

        response
    }
}

pub struct WLButton<'a> {
    wl: WidgetLayout<'a>,
}

impl<'a> WLButton<'a> {
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            wl: WidgetLayout::new()
                .sense(Sense::click())
                .add(Item::default(), WidgetLayoutItemType::Text(text.into())),
        }
    }

    pub fn image(image: impl Into<Image<'a>>) -> Self {
        Self {
            wl: WidgetLayout::new().sense(Sense::click()).add(
                Item::default(),
                WidgetLayoutItemType::Image(image.into().max_size(Vec2::splat(16.0))),
            ),
        }
    }

    pub fn image_and_text(image: impl Into<Image<'a>>, text: impl Into<WidgetText>) -> Self {
        Self {
            wl: WidgetLayout::new()
                .sense(Sense::click())
                .add(Item::default(), WidgetLayoutItemType::Image(image.into()))
                .add(Item::default(), WidgetLayoutItemType::Text(text.into())),
        }
    }

    pub fn right_text(mut self, text: impl Into<WidgetText>) -> Self {
        self.wl = self
            .wl
            .add(Item::default(), WidgetLayoutItemType::Grow)
            .add(Item::default(), WidgetLayoutItemType::Text(text.into()));
        self
    }
}

impl<'a> Widget for WLButton<'a> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let response = ui.ctx().read_response(ui.next_auto_id());

        let visuals = response.map_or(&ui.style().visuals.widgets.inactive, |response| {
            ui.style().interact(&response)
        });

        self.wl.frame = self
            .wl
            .frame
            .inner_margin(ui.style().spacing.button_padding)
            .fill(visuals.bg_fill)
            .stroke(visuals.bg_stroke)
            .corner_radius(visuals.corner_radius);

        self.wl.show(ui)
    }
}
