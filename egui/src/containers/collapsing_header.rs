use std::hash::Hash;

use crate::{
    layout::Direction,
    paint::{PaintCmd, TextStyle},
    widgets::Label,
    *,
};

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub(crate) struct State {
    open: bool,

    /// Height of the region when open. Used for animations
    open_height: Option<f32>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            open: false,
            open_height: None,
        }
    }
}

impl State {
    pub fn from_memory_with_default_open(ctx: &Context, id: Id, default_open: bool) -> Self {
        *ctx.memory().collapsing_headers.entry(id).or_insert(State {
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
        ui.ctx().request_repaint();
    }

    /// 0 for closed, 1 for open, with tweening
    pub fn openness(&self, ctx: &Context, id: Id) -> f32 {
        ctx.animate_bool(id, self.open)
    }

    /// Show contents if we are open, with a nice animation between closed and open
    pub fn add_contents<R>(
        &mut self,
        ui: &mut Ui,
        id: Id,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<(R, Rect)> {
        let openness = self.openness(ui.ctx(), id);
        let animate = 0.0 < openness && openness < 1.0;
        if animate {
            Some(ui.add_custom(|child_ui| {
                let max_height = if self.open {
                    if let Some(full_height) = self.open_height {
                        remap_clamp(openness, 0.0..=1.0, 0.0..=full_height)
                    } else {
                        // First frame of expansion.
                        // We don't know full height yet, but we will next frame.
                        // Just use a placeholder value that shows some movement:
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
            let ret_rect = ui.add_custom(add_contents);
            let full_size = ret_rect.1.size();
            self.open_height = Some(full_size.y);
            Some(ret_rect)
        } else {
            None
        }
    }
}

/// Paint the arrow icon that indicated if the region is open or not
pub fn paint_icon(ui: &mut Ui, openness: f32, response: &Response) {
    let stroke = ui.style().interact(response).fg_stroke;

    let rect = response.rect;

    // Draw a pointy triangle arrow:
    let rect = Rect::from_center_size(rect.center(), vec2(rect.width(), rect.height()) * 0.75);
    let mut points = vec![rect.left_top(), rect.right_top(), rect.center_bottom()];
    let rotation = Vec2::angled(remap(openness, 0.0..=1.0, -TAU / 4.0..=0.0));
    for p in &mut points {
        let v = *p - rect.center();
        let v = rotation.rotate_other(v);
        *p = rect.center() + v;
    }

    ui.painter().add(PaintCmd::Path {
        points,
        closed: true,
        fill: Default::default(),
        stroke,
    });
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
        let text_pos = available.min + vec2(ui.style().spacing.indent, 0.0);
        let galley = label.layout_width(ui, available.right() - text_pos.x);
        let text_max_x = text_pos.x + galley.size.x;
        let desired_width = text_max_x - available.left();
        let desired_width = desired_width.max(available.width());

        let mut desired_size = vec2(
            desired_width,
            galley.size.y + 2.0 * ui.style().spacing.button_padding.y,
        );
        desired_size.y = desired_size.y.max(ui.style().spacing.clickable_diameter);
        let rect = ui.allocate_space(desired_size);
        let rect = rect.expand2(ui.style().spacing.button_expand);

        let response = ui.interact(rect, id, Sense::click());
        let text_pos = pos2(text_pos.x, response.rect.center().y - galley.size.y / 2.0);

        let mut state = State::from_memory_with_default_open(ui.ctx(), id, default_open);
        if response.clicked {
            state.toggle(ui);
        }

        let bg_index = ui.painter().add(PaintCmd::Noop);

        {
            let (mut icon_rect, _) = ui.style().spacing.icon_rectangles(response.rect);
            icon_rect.set_center(pos2(
                response.rect.left() + ui.style().spacing.indent / 2.0,
                response.rect.center().y,
            ));
            let icon_response = Response {
                rect: icon_rect,
                ..response.clone()
            };
            let openness = state.openness(ui.ctx(), id);
            paint_icon(ui, openness, &icon_response);
        }

        let painter = ui.painter();
        painter.galley(
            text_pos,
            galley,
            label.text_style_or_default(ui.style()),
            ui.style().interact(&response).text_color(),
        );

        painter.set(
            bg_index,
            PaintCmd::Rect {
                rect: response.rect,
                corner_radius: ui.style().interact(&response).corner_radius,
                fill: ui.style().interact(&response).bg_fill,
                stroke: Default::default(),
            },
        );

        Prepared { id, state }
    }

    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let Prepared { id, mut state } = self.begin(ui);
        let ret_response = state.add_contents(ui, id, |ui| ui.indent(id, add_contents).0);
        let ret = ret_response.map(|ri| ri.0);
        ui.memory().collapsing_headers.insert(id, state);
        ret
    }
}
