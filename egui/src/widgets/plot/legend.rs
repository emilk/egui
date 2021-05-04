use std::string::String;

use crate::*;

pub(crate) struct LegendEntry {
    pub text: String,
    pub color: Color32,
    pub checked: bool,
    pub hovered: bool,
}

impl LegendEntry {
    pub fn new(text: String, color: Color32, checked: bool) -> Self {
        Self {
            text,
            color,
            checked,
            hovered: false,
        }
    }
}

struct LegendEntryWidget<'a> {
    entry: &'a mut LegendEntry,
}

impl<'a> LegendEntryWidget<'a> {
    fn new(entry: &'a mut LegendEntry) -> Self {
        Self { entry }
    }
}

impl<'a> Widget for LegendEntryWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let LegendEntry {
            checked,
            text,
            color,
            ..
        } = self.entry;
        let icon_width = ui.spacing().icon_width;
        let icon_spacing = ui.spacing().icon_spacing;
        let padding = vec2(2.0, 2.0);
        let total_extra = padding + vec2(icon_width + icon_spacing, 0.0) + padding;

        let text_style = TextStyle::Button;
        let galley = ui.fonts().layout_no_wrap(text_style, text.clone());

        let mut desired_size = total_extra + galley.size;
        desired_size = desired_size.at_least(ui.spacing().interact_size);
        desired_size.y = desired_size.y.at_least(icon_width);

        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());
            let rect = rect.shrink2(padding);

            response.widget_info(|| {
                WidgetInfo::selected(WidgetType::RadioButton, *checked, &galley.text)
            });

            let visuals = ui.style().interact(&response);

            let (small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);

            let painter = ui.painter();

            painter.add(Shape::Circle {
                center: big_icon_rect.center(),
                radius: big_icon_rect.width() / 2.0 + visuals.expansion,
                fill: visuals.bg_fill,
                stroke: visuals.bg_stroke,
            });

            if *checked {
                painter.add(Shape::Circle {
                    center: small_icon_rect.center(),
                    radius: small_icon_rect.width() * 0.8,
                    fill: *color,
                    stroke: Default::default(),
                });
            }

            let text_position = pos2(
                rect.left() + padding.x + icon_width + icon_spacing,
                rect.center().y - 0.5 * galley.size.y,
            );
            painter.galley(text_position, galley, visuals.text_color());
            response
        })
        .inner
    }
}

pub(crate) struct PlotLegend<'e> {
    pub entries: &'e mut [LegendEntry],
    max_size: Rect,
}

impl<'e> PlotLegend<'e> {
    pub fn new(entries: &'e mut [LegendEntry], max_size: Rect) -> Self {
        Self { entries, max_size }
    }
}

impl Widget for PlotLegend<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let PlotLegend { entries, max_size } = self;

        let mut rect = max_size;

        entries
            .iter_mut()
            .map(|entry| {
                let reference = LegendEntryWidget::new(entry);
                let response = ui.put(rect, reference);
                entry.checked ^= response.clicked_by(PointerButton::Primary);
                entry.hovered = response.hovered();
                rect.min.y += response.rect.height();
                response
            })
            .reduce(|last, current| last.union(current))
            .unwrap()
    }
}
