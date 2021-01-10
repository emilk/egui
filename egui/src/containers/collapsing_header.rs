use std::hash::Hash;

use crate::{
    paint::{Shape, TextStyle},
    widgets::Label,
    *,
};

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
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
        if ctx.memory().everything_is_visible() {
            Some(true)
        } else {
            ctx.memory()
                .collapsing_headers
                .get(&id)
                .map(|state| state.open)
        }
    }

    pub fn toggle(&mut self, ui: &Ui) {
        self.open = !self.open;
        ui.ctx().request_repaint();
    }

    /// 0 for closed, 1 for open, with tweening
    pub fn openness(&self, ctx: &Context, id: Id) -> f32 {
        if ctx.memory().everything_is_visible() {
            1.0
        } else {
            ctx.animate_bool(id, self.open)
        }
    }

    /// Show contents if we are open, with a nice animation between closed and open
    pub fn add_contents<R>(
        &mut self,
        ui: &mut Ui,
        id: Id,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<(R, Response)> {
        let openness = self.openness(ui.ctx(), id);
        if openness <= 0.0 {
            None
        } else if openness < 1.0 {
            Some(ui.wrap(|child_ui| {
                let max_height = if self.open && self.open_height.is_none() {
                    // First frame of expansion.
                    // We don't know full height yet, but we will next frame.
                    // Just use a placeholder value that shows some movement:
                    10.0
                } else {
                    let full_height = self.open_height.unwrap_or_default();
                    remap_clamp(openness, 0.0..=1.0, 0.0..=full_height)
                };

                let mut clip_rect = child_ui.clip_rect();
                clip_rect.max.y = clip_rect.max.y.min(child_ui.max_rect().top() + max_height);
                child_ui.set_clip_rect(clip_rect);

                let ret = add_contents(child_ui);

                let mut min_rect = child_ui.min_rect();
                self.open_height = Some(min_rect.height());

                // Pretend children took up at most `max_height` space:
                min_rect.max.y = min_rect.max.y.at_most(min_rect.top() + max_height);
                child_ui.force_set_min_rect(min_rect);
                ret
            }))
        } else {
            let (ret, response) = ui.wrap(add_contents);
            let full_size = response.rect.size();
            self.open_height = Some(full_size.y);
            Some((ret, response))
        }
    }
}

/// Paint the arrow icon that indicated if the region is open or not
pub(crate) fn paint_icon(ui: &mut Ui, openness: f32, response: &Response) {
    let stroke = ui.style().interact(response).fg_stroke;

    let rect = response.rect;

    // Draw a pointy triangle arrow:
    let rect = Rect::from_center_size(rect.center(), vec2(rect.width(), rect.height()) * 0.75);
    let mut points = vec![rect.left_top(), rect.right_top(), rect.center_bottom()];
    use std::f32::consts::TAU;
    let rotation = math::Rot2::from_angle(remap(openness, 0.0..=1.0, -TAU / 4.0..=0.0));
    for p in &mut points {
        *p = rect.center() + rotation * (*p - rect.center());
    }

    ui.painter().add(Shape::closed_line(points, stroke));
}

/// A header which can be collapsed/expanded, revealing a contained [`Ui`] region.
pub struct CollapsingHeader {
    label: Label,
    default_open: bool,
    id_source: Id,
}

impl CollapsingHeader {
    /// The `CollapsingHeader` starts out collapsed unless you call `default_open`.
    pub fn new(label: impl Into<String>) -> Self {
        let label = Label::new(label)
            .text_style(TextStyle::Button)
            .multiline(false);
        let id_source = Id::new(label.text());
        Self {
            label,
            default_open: false,
            id_source,
        }
    }

    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Explicitly set the source of the `Id` of this widget, instead of using title label.
    /// This is useful if the title label is dynamic or not unique.
    pub fn id_source(mut self, id_source: impl Hash) -> Self {
        self.id_source = Id::new(id_source);
        self
    }
}

struct Prepared {
    id: Id,
    header_response: Response,
    state: State,
}

impl CollapsingHeader {
    fn begin(self, ui: &mut Ui) -> Prepared {
        assert!(
            ui.layout().main_dir().is_vertical(),
            "Horizontal collapsing is unimplemented"
        );
        let Self {
            label,
            default_open,
            id_source,
        } = self;

        // TODO: horizontal layout, with icon and text as labels. Insert background behind using Frame.

        let id = ui.make_persistent_id(id_source);

        let available = ui.available_rect_before_wrap_finite();
        let text_pos = available.min + vec2(ui.style().spacing.indent, 0.0);
        let galley = label.layout_width(ui, available.right() - text_pos.x);
        let text_max_x = text_pos.x + galley.size.x;
        let desired_width = text_max_x - available.left();
        let desired_width = desired_width.max(available.width());

        let mut desired_size = vec2(
            desired_width,
            galley.size.y + 2.0 * ui.style().spacing.button_padding.y,
        );
        desired_size = desired_size.at_least(ui.style().spacing.interact_size);
        let (_, rect) = ui.allocate_space(desired_size);

        let header_response = ui.interact(rect, id, Sense::click());
        let text_pos = pos2(
            text_pos.x,
            header_response.rect.center().y - galley.size.y / 2.0,
        );

        let mut state = State::from_memory_with_default_open(ui.ctx(), id, default_open);
        if header_response.clicked {
            state.toggle(ui);
        }

        let bg_index = ui.painter().add(Shape::Noop);

        {
            let (mut icon_rect, _) = ui.style().spacing.icon_rectangles(header_response.rect);
            icon_rect.set_center(pos2(
                header_response.rect.left() + ui.style().spacing.indent / 2.0,
                header_response.rect.center().y,
            ));
            let icon_response = Response {
                rect: icon_rect,
                ..header_response.clone()
            };
            let openness = state.openness(ui.ctx(), id);
            paint_icon(ui, openness, &icon_response);
        }

        let painter = ui.painter();
        painter.galley(
            text_pos,
            galley,
            label.text_style_or_default(ui.style()),
            ui.style().interact(&header_response).text_color(),
        );

        painter.set(
            bg_index,
            Shape::Rect {
                rect: header_response.rect,
                corner_radius: ui.style().interact(&header_response).corner_radius,
                fill: ui.style().interact(&header_response).bg_fill,
                stroke: Default::default(),
            },
        );

        Prepared {
            id,
            header_response,
            state,
        }
    }

    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> CollapsingResponse<R> {
        let Prepared {
            id,
            header_response,
            mut state,
        } = self.begin(ui);
        let ret_response = state.add_contents(ui, id, |ui| ui.indent(id, add_contents).0);
        ui.memory().collapsing_headers.insert(id, state);

        if let Some((ret, response)) = ret_response {
            CollapsingResponse {
                header_response,
                body_response: Some(response),
                body_returned: Some(ret),
            }
        } else {
            CollapsingResponse {
                header_response,
                body_response: None,
                body_returned: None,
            }
        }
    }
}

/// The response from showing a [`CollapsingHeader`].
pub struct CollapsingResponse<R> {
    pub header_response: Response,
    /// None iff collapsed.
    pub body_response: Option<Response>,
    /// None iff collapsed.
    pub body_returned: Option<R>,
}
