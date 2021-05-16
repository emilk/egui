use std::{
    collections::{BTreeMap, HashSet},
    string::String,
};

use super::Curve;
use crate::*;

/// Where to place the plot legend.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LegendPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl LegendPosition {
    pub fn all() -> impl Iterator<Item = LegendPosition> {
        [
            LegendPosition::TopLeft,
            LegendPosition::TopRight,
            LegendPosition::BottomLeft,
            LegendPosition::BottomRight,
        ]
        .iter()
        .copied()
    }
}

/// The configuration for a plot legend.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, PartialEq)]
pub struct Legend {
    pub text_style: TextStyle,
    pub position: LegendPosition,
}

impl Default for Legend {
    fn default() -> Self {
        Self {
            text_style: TextStyle::Body,
            position: LegendPosition::TopRight,
        }
    }
}

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone)]
struct LegendEntry {
    config: Legend,
    color: Option<Color32>,
    checked: bool,
    hovered: bool,
}

impl LegendEntry {
    fn new(config: Legend, color: Option<Color32>, checked: bool) -> Self {
        Self {
            config,
            color,
            checked,
            hovered: false,
        }
    }
}

impl Widget for (&String, &mut LegendEntry) {
    fn ui(self, ui: &mut Ui) -> Response {
        let (
            text,
            LegendEntry {
                config,
                color,
                checked,
                hovered,
            },
        ) = self;

        let galley = ui.fonts().layout_no_wrap(config.text_style, text.clone());

        let icon_size = galley.size.y;
        let icon_spacing = icon_size / 5.0;
        let total_extra = vec2(icon_size + icon_spacing, 0.0);

        let desired_size = total_extra + galley.size;
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, *checked, &galley.text));

        let visuals = ui.style().interact(&response);

        let icon_position_x = match config.position {
            LegendPosition::BottomLeft | LegendPosition::TopLeft => rect.left() + icon_size / 2.0,
            LegendPosition::BottomRight | LegendPosition::TopRight => {
                rect.right() - icon_size / 2.0
            }
        };
        let icon_position = pos2(icon_position_x, rect.center().y);
        let icon_rect = Rect::from_center_size(icon_position, vec2(icon_size, icon_size));

        let painter = ui.painter();

        painter.add(Shape::Circle {
            center: icon_rect.center(),
            radius: icon_size * 0.5,
            fill: visuals.bg_fill,
            stroke: visuals.bg_stroke,
        });

        if *checked {
            let neutral_color = ui.visuals().noninteractive().fg_stroke.color;
            painter.add(Shape::Circle {
                center: icon_rect.center(),
                radius: icon_size * 0.4,
                fill: color.unwrap_or(neutral_color),
                stroke: Default::default(),
            });
        }

        let text_position_x = match config.position {
            LegendPosition::BottomLeft | LegendPosition::TopLeft => {
                rect.left() + icon_size + icon_spacing
            }
            LegendPosition::BottomRight | LegendPosition::TopRight => {
                rect.right() - icon_size - icon_spacing - galley.size.x
            }
        };
        let text_position = pos2(text_position_x, rect.center().y - 0.5 * galley.size.y);
        painter.galley(text_position, galley, visuals.text_color());

        *checked ^= response.clicked_by(PointerButton::Primary);
        *hovered = response.hovered();

        response
    }
}

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone)]
pub(crate) struct LegendWidget {
    rect: Rect,
    entries: BTreeMap<String, LegendEntry>,
    config: Legend,
}

impl LegendWidget {
    /// Create a new legend from curves, the names of curves that are hidden and the style of the
    /// text. Returns `None` if the legend has no entries.
    pub fn try_new(
        rect: Rect,
        config: Legend,
        curves: &[Curve],
        hidden_curves: &HashSet<String>,
    ) -> Option<Self> {
        // Collect the legend entries. If multiple curves have the same name, they share a
        // checkbox. If their colors don't match, we pick a neutral color for the checkbox.
        let mut entries: BTreeMap<String, LegendEntry> = BTreeMap::new();
        curves
            .iter()
            .filter(|curve| !curve.name.is_empty())
            .for_each(|curve| {
                entries
                    .entry(curve.name.clone())
                    .and_modify(|entry| {
                        if entry.color != curve.get_color() {
                            entry.color = None
                        }
                    })
                    .or_insert_with(|| {
                        let color = curve.get_color();
                        let checked = !hidden_curves.contains(&curve.name);
                        LegendEntry::new(config, color, checked)
                    });
            });
        (!entries.is_empty()).then(|| Self {
            rect,
            entries,
            config,
        })
    }

    // Get the names of the hidden curves.
    pub fn get_hidden_curves(&self) -> HashSet<String> {
        self.entries
            .iter()
            .filter(|(_, entry)| !entry.checked)
            .map(|(name, _)| name.clone())
            .collect()
    }

    // Get the name of the hovered curve.
    pub fn get_hovered_entry_name(&self) -> Option<String> {
        self.entries
            .iter()
            .find(|(_, entry)| entry.hovered)
            .map(|(name, _)| name.to_string())
    }
}

impl Widget for &mut LegendWidget {
    fn ui(self, ui: &mut Ui) -> Response {
        let main_dir = match self.config.position {
            LegendPosition::TopLeft | LegendPosition::TopRight => Direction::TopDown,
            LegendPosition::BottomLeft | LegendPosition::BottomRight => Direction::BottomUp,
        };
        let cross_align = match self.config.position {
            LegendPosition::TopLeft | LegendPosition::BottomLeft => Align::LEFT,
            LegendPosition::TopRight | LegendPosition::BottomRight => Align::RIGHT,
        };
        let layout = Layout::from_main_dir_and_cross_align(main_dir, cross_align);
        let legend_pad = 2.0;
        let legend_rect = self.rect.shrink(legend_pad);
        let mut legend_ui = ui.child_ui(legend_rect, layout);
        self.entries
            .iter_mut()
            .map(|entry| legend_ui.add(entry))
            .reduce(|r1, r2| r1.union(r2))
            .unwrap()
    }
}
