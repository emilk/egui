use std::{collections::BTreeMap, string::String};

use crate::*;

use super::items::PlotItem;

/// Where to place the plot legend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corner {
    LeftTop,
    RightTop,
    LeftBottom,
    RightBottom,
}

impl Corner {
    pub fn all() -> impl Iterator<Item = Corner> {
        [
            Corner::LeftTop,
            Corner::RightTop,
            Corner::LeftBottom,
            Corner::RightBottom,
        ]
        .iter()
        .copied()
    }
}

/// The configuration for a plot legend.
#[derive(Clone, PartialEq)]
pub struct Legend {
    pub text_style: TextStyle,
    pub background_alpha: f32,
    pub position: Corner,
}

impl Default for Legend {
    fn default() -> Self {
        Self {
            text_style: TextStyle::Body,
            background_alpha: 0.75,
            position: Corner::RightTop,
        }
    }
}

impl Legend {
    /// Which text style to use for the legend. Default: `TextStyle::Body`.
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = style;
        self
    }

    /// The alpha of the legend background. Default: `0.75`.
    pub fn background_alpha(mut self, alpha: f32) -> Self {
        self.background_alpha = alpha;
        self
    }

    /// In which corner to place the legend. Default: `Corner::RightTop`.
    pub fn position(mut self, corner: Corner) -> Self {
        self.position = corner;
        self
    }
}

#[derive(Clone)]
struct LegendEntry {
    color: Color32,
    checked: bool,
    hovered: bool,
}

impl LegendEntry {
    fn new(color: Color32, checked: bool) -> Self {
        Self {
            color,
            checked,
            hovered: false,
        }
    }

    fn ui(&mut self, ui: &mut Ui, text: String, text_style: &TextStyle) -> Response {
        let Self {
            color,
            checked,
            hovered,
        } = self;

        let font_id = text_style.resolve(ui.style());

        let galley = ui.fonts(|f| f.layout_delayed_color(text, font_id, f32::INFINITY));

        let icon_size = galley.size().y;
        let icon_spacing = icon_size / 5.0;
        let total_extra = vec2(icon_size + icon_spacing, 0.0);

        let desired_size = total_extra + galley.size();
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        response
            .widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, *checked, galley.text()));

        let visuals = ui.style().interact(&response);
        let label_on_the_left = ui.layout().horizontal_placement() == Align::RIGHT;

        let icon_position_x = if label_on_the_left {
            rect.right() - icon_size / 2.0
        } else {
            rect.left() + icon_size / 2.0
        };
        let icon_position = pos2(icon_position_x, rect.center().y);
        let icon_rect = Rect::from_center_size(icon_position, vec2(icon_size, icon_size));

        let painter = ui.painter();

        painter.add(epaint::CircleShape {
            center: icon_rect.center(),
            radius: icon_size * 0.5,
            fill: visuals.bg_fill,
            stroke: visuals.bg_stroke,
        });

        if *checked {
            let fill = if *color == Color32::TRANSPARENT {
                ui.visuals().noninteractive().fg_stroke.color
            } else {
                *color
            };
            painter.add(epaint::Shape::circle_filled(
                icon_rect.center(),
                icon_size * 0.4,
                fill,
            ));
        }

        let text_position_x = if label_on_the_left {
            rect.right() - icon_size - icon_spacing - galley.size().x
        } else {
            rect.left() + icon_size + icon_spacing
        };

        let text_position = pos2(text_position_x, rect.center().y - 0.5 * galley.size().y);
        painter.galley_with_color(text_position, galley, visuals.text_color());

        *checked ^= response.clicked_by(PointerButton::Primary);
        *hovered = response.hovered();

        response
    }
}

#[derive(Clone)]
pub(super) struct LegendWidget {
    rect: Rect,
    entries: BTreeMap<String, LegendEntry>,
    config: Legend,
}

impl LegendWidget {
    /// Create a new legend from items, the names of items that are hidden and the style of the
    /// text. Returns `None` if the legend has no entries.
    pub(super) fn try_new(
        rect: Rect,
        config: Legend,
        items: &[Box<dyn PlotItem>],
        hidden_items: &ahash::HashSet<String>,
    ) -> Option<Self> {
        // Collect the legend entries. If multiple items have the same name, they share a
        // checkbox. If their colors don't match, we pick a neutral color for the checkbox.
        let mut entries: BTreeMap<String, LegendEntry> = BTreeMap::new();
        items
            .iter()
            .filter(|item| !item.name().is_empty())
            .for_each(|item| {
                entries
                    .entry(item.name().to_owned())
                    .and_modify(|entry| {
                        if entry.color != item.color() {
                            // Multiple items with different colors
                            entry.color = Color32::TRANSPARENT;
                        }
                    })
                    .or_insert_with(|| {
                        let color = item.color();
                        let checked = !hidden_items.contains(item.name());
                        LegendEntry::new(color, checked)
                    });
            });
        (!entries.is_empty()).then_some(Self {
            rect,
            entries,
            config,
        })
    }

    // Get the names of the hidden items.
    pub fn hidden_items(&self) -> ahash::HashSet<String> {
        self.entries
            .iter()
            .filter(|(_, entry)| !entry.checked)
            .map(|(name, _)| name.clone())
            .collect()
    }

    // Get the name of the hovered items.
    pub fn hovered_entry_name(&self) -> Option<String> {
        self.entries
            .iter()
            .find(|(_, entry)| entry.hovered)
            .map(|(name, _)| name.to_string())
    }
}

impl Widget for &mut LegendWidget {
    fn ui(self, ui: &mut Ui) -> Response {
        let LegendWidget {
            rect,
            entries,
            config,
        } = self;

        let main_dir = match config.position {
            Corner::LeftTop | Corner::RightTop => Direction::TopDown,
            Corner::LeftBottom | Corner::RightBottom => Direction::BottomUp,
        };
        let cross_align = match config.position {
            Corner::LeftTop | Corner::LeftBottom => Align::LEFT,
            Corner::RightTop | Corner::RightBottom => Align::RIGHT,
        };
        let layout = Layout::from_main_dir_and_cross_align(main_dir, cross_align);
        let legend_pad = 4.0;
        let legend_rect = rect.shrink(legend_pad);
        let mut legend_ui = ui.child_ui(legend_rect, layout);
        legend_ui
            .scope(|ui| {
                let background_frame = Frame {
                    inner_margin: vec2(8.0, 4.0).into(),
                    rounding: ui.style().visuals.window_rounding,
                    shadow: epaint::Shadow::NONE,
                    fill: ui.style().visuals.extreme_bg_color,
                    stroke: ui.style().visuals.window_stroke(),
                    ..Default::default()
                }
                .multiply_with_opacity(config.background_alpha);
                background_frame
                    .show(ui, |ui| {
                        entries
                            .iter_mut()
                            .map(|(name, entry)| entry.ui(ui, name.clone(), &config.text_style))
                            .reduce(|r1, r2| r1.union(r2))
                            .unwrap()
                    })
                    .inner
            })
            .inner
    }
}
