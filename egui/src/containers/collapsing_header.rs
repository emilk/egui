use std::hash::Hash;

use crate::{
    layout::Direction,
    paint::{LineStyle, PaintCmd, Path, TextStyle},
    widgets::Label,
    *,
};

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub(crate) struct State {
    open: bool,

    // Times are relative, and we don't want to continue animations anyway, hence `serde(skip)`
    #[cfg_attr(feature = "serde", serde(skip))]
    toggle_time: f64,

    /// Height of the region when open. Used for animations
    open_height: Option<f32>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            open: false,
            toggle_time: -f64::INFINITY,
            open_height: None,
        }
    }
}

impl State {
    pub fn from_memory_with_default_open(ui: &Ui, id: Id, default_open: bool) -> Self {
        *ui.memory().collapsing_headers.entry(id).or_insert(State {
            open: default_open,
            ..Default::default()
        })
    }

    // Helper
    pub fn is_open(ctx: &Context, id: Id) -> Option<bool> {
        ctx.memory()
            .collapsing_headers
            .get(&id)
            .map(|state| state.open)
    }

    pub fn toggle(&mut self, ui: &Ui) {
        self.open = !self.open;
        self.toggle_time = ui.input().time;
        ui.ctx().request_repaint();
    }

    /// 0 for closed, 1 for open, with tweening
    pub fn openness(&self, ui: &Ui) -> f32 {
        let animation_time = ui.style().animation_time;
        let time_since_toggle = (ui.input().time - self.toggle_time) as f32;
        let time_since_toggle = time_since_toggle + ui.input().predicted_dt; // Instant feedback
        if time_since_toggle <= animation_time {
            ui.ctx().request_repaint();
        }
        if self.open {
            remap_clamp(time_since_toggle, 0.0..=animation_time, 0.0..=1.0)
        } else {
            remap_clamp(time_since_toggle, 0.0..=animation_time, 1.0..=0.0)
        }
    }

    /// Paint the arrow icon that indicated if the region is open or not
    pub fn paint_icon(&self, ui: &mut Ui, interact: &InteractInfo) {
        let stroke_color = ui.style().interact(interact).stroke_color;
        let stroke_width = ui.style().interact(interact).stroke_width;

        let rect = interact.rect;

        let openness = self.openness(ui);

        // Draw a pointy triangle arrow:
        let rect = Rect::from_center_size(rect.center(), vec2(rect.width(), rect.height()) * 0.75);
        let mut points = [rect.left_top(), rect.right_top(), rect.center_bottom()];
        let rotation = Vec2::angled(remap(openness, 0.0..=1.0, -TAU / 4.0..=0.0));
        for p in &mut points {
            let v = *p - rect.center();
            let v = rotation.rotate_other(v);
            *p = rect.center() + v;
        }

        ui.painter().add(PaintCmd::Path {
            path: Path::from_point_loop(&points),
            closed: true,
            fill: None,
            outline: Some(LineStyle::new(stroke_width, stroke_color)),
        });
    }

    /// Show contents if we are open, with a nice animation between closed and open
    pub fn add_contents<R>(
        &mut self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<(R, Rect)> {
        let openness = self.openness(ui);
        let animate = 0.0 < openness && openness < 1.0;
        if animate {
            ui.ctx().request_repaint();
            Some(ui.add_custom(|child_ui| {
                let max_height = if self.open {
                    if let Some(full_height) = self.open_height {
                        remap_clamp(openness, 0.0..=1.0, 0.0..=full_height)
                    } else {
                        // First frame of expansion.
                        // We don't know full height yet, but we will next frame.
                        // Just use a placehodler value that shows some movement:
                        10.0
                    }
                } else {
                    let full_height = self.open_height.unwrap_or_default();
                    remap_clamp(openness, 0.0..=1.0, 0.0..=full_height)
                };

                let mut clip_rect = child_ui.clip_rect();
                clip_rect.max.y = clip_rect.max.y.min(child_ui.rect().top() + max_height);
                child_ui.set_clip_rect(clip_rect);

                let top_left = child_ui.top_left();
                let r = add_contents(child_ui);

                self.open_height = Some(child_ui.bounding_size().y);

                // Pretend children took up less space:
                let mut child_bounds = child_ui.child_bounds();
                child_bounds.max.y = child_bounds.max.y.min(top_left.y + max_height);
                child_ui.force_set_child_bounds(child_bounds);
                r
            }))
        } else if self.open {
            let r_interact = ui.add_custom(add_contents);
            let full_size = r_interact.1.size();
            self.open_height = Some(full_size.y);
            Some(r_interact)
        } else {
            None
        }
    }
}

/// A header which can be collapsed/expanded, revealing a contained `Ui` region.
pub struct CollapsingHeader {
    label: Label,
    default_open: bool,
    id_source: Option<Id>,
}

impl CollapsingHeader {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: Label::new(label)
                .text_style(TextStyle::Button)
                .multiline(false),
            default_open: false,
            id_source: None,
        }
    }

    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Explicitly set the source of the `Id` of this widget, instead of using title label.
    /// This is useful if the title label is dynamics.
    pub fn id_source(mut self, id_source: impl Hash) -> Self {
        self.id_source = Some(Id::new(id_source));
        self
    }
}

struct Prepared {
    id: Id,
    state: State,
}

impl CollapsingHeader {
    fn begin(self, ui: &mut Ui) -> Prepared {
        assert!(
            ui.layout().dir() == Direction::Vertical,
            "Horizontal collapsing is unimplemented"
        );
        let Self {
            label,
            default_open,
            id_source,
        } = self;

        // TODO: horizontal layout, with icon and text as labels. Insert background behind using Frame.

        let title = label.text();
        let id = ui.make_unique_child_id_full(id_source, Some(title));

        let available = ui.available_finite();
        let text_pos = available.min + vec2(ui.style().indent, 0.0);
        let galley = label.layout_width(ui, available.width() - ui.style().indent);
        let text_max_x = text_pos.x + galley.size.x;
        let desired_width = text_max_x - available.left();
        let desired_width = desired_width.max(available.width());

        let size = vec2(
            desired_width,
            galley.size.y + 2.0 * ui.style().button_padding.y,
        );

        let rect = ui.allocate_space(size);
        let interact = ui.interact(rect, id, Sense::click());
        let text_pos = pos2(text_pos.x, interact.rect.center().y - galley.size.y / 2.0);

        let mut state = State::from_memory_with_default_open(ui, id, default_open);
        if interact.clicked {
            state.toggle(ui);
        }

        let bg_index = ui.painter().add(PaintCmd::Noop);

        {
            let (mut icon_rect, _) = ui.style().icon_rectangles(interact.rect);
            icon_rect.set_center(pos2(
                interact.rect.left() + ui.style().indent / 2.0,
                interact.rect.center().y,
            ));
            let icon_interact = InteractInfo {
                rect: icon_rect,
                ..interact
            };
            state.paint_icon(ui, &icon_interact);
        }

        let painter = ui.painter();
        painter.galley(
            text_pos,
            galley,
            label.text_style,
            ui.style().interact(&interact).stroke_color,
        );

        painter.set(
            bg_index,
            PaintCmd::Rect {
                corner_radius: ui.style().interact(&interact).corner_radius,
                fill: ui.style().interact(&interact).bg_fill,
                outline: None,
                rect: interact.rect,
            },
        );

        Prepared { id, state }
    }

    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let Prepared { id, mut state } = self.begin(ui);
        let r_interact = state.add_contents(ui, |ui| ui.indent(id, add_contents).0);
        let ret = r_interact.map(|ri| ri.0);
        ui.memory().collapsing_headers.insert(id, state);
        ret
    }
}
