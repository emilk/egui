use std::hash::Hash;

use crate::{widgets::Label, *};
use epaint::{Shape, TextStyle};

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
        *ctx.memory().id_data.get_or_insert_with(id, || State {
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
                .id_data
                .get::<State>(&id)
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
    ) -> Option<InnerResponse<R>> {
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
            let ret_response = ui.wrap(add_contents);
            let full_size = ret_response.response.rect.size();
            self.open_height = Some(full_size.y);
            Some(ret_response)
        }
    }
}

/// Paint the arrow icon that indicated if the region is open or not
pub(crate) fn paint_icon(ui: &mut Ui, openness: f32, response: &Response) {
    let visuals = ui.style().interact(response);
    let stroke = visuals.fg_stroke;

    let rect = response.rect;

    // Draw a pointy triangle arrow:
    let rect = Rect::from_center_size(rect.center(), vec2(rect.width(), rect.height()) * 0.75);
    let rect = rect.expand(visuals.expansion);
    let mut points = vec![rect.left_top(), rect.right_top(), rect.center_bottom()];
    use std::f32::consts::TAU;
    let rotation = emath::Rot2::from_angle(remap(openness, 0.0..=1.0, -TAU / 4.0..=0.0));
    for p in &mut points {
        *p = rect.center() + rotation * (*p - rect.center());
    }

    ui.painter().add(Shape::closed_line(points, stroke));
}

/// A header which can be collapsed/expanded, revealing a contained [`Ui`] region.
///
///
/// ```
/// # let ui = &mut egui::Ui::__test();
/// egui::CollapsingHeader::new("Heading")
///     .show(ui, |ui| {
///         ui.label("Contents");
///     });
///
/// // Short version:
/// ui.collapsing("Heading", |ui| { ui.label("Contents"); });
/// ```
pub struct CollapsingHeader {
    label: Label,
    default_open: bool,
    id_source: Id,
    enabled: bool,
}

impl CollapsingHeader {
    /// The `CollapsingHeader` starts out collapsed unless you call `default_open`.
    ///
    /// The label is used as an [`Id`] source.
    /// If the label is unique and static this is fine,
    /// but if it changes or there are several `CollapsingHeader` with the same title
    /// you need to provide a unique id source with [`Self::id_source`].
    pub fn new(label: impl Into<String>) -> Self {
        let label = Label::new(label).text_style(TextStyle::Button).wrap(false);
        let id_source = Id::new(label.text());
        Self {
            label,
            default_open: false,
            id_source,
            enabled: true,
        }
    }

    /// By default, the `CollapsingHeader` is collapsed.
    /// Call `.default_open(true)` to change this.
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

    /// By default, the `CollapsingHeader` text style is `TextStyle::Button`.
    /// Call `.text_style(style)` to change this.
    pub fn text_style(mut self, text_style: TextStyle) -> Self {
        self.label = self.label.text_style(text_style);
        self
    }

    /// If you set this to `false`, the `CollapsingHeader` will be grayed out and un-clickable.
    ///
    /// This is a convenience for [`Ui::set_enabled`].
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
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
            enabled: _,
        } = self;

        // TODO: horizontal layout, with icon and text as labels. Insert background behind using Frame.

        let id = ui.make_persistent_id(id_source);
        let button_padding = ui.spacing().button_padding;

        let available = ui.available_rect_before_wrap_finite();
        let text_pos = available.min + vec2(ui.spacing().indent, 0.0);
        let galley = label.layout_width(ui, available.right() - text_pos.x);
        let text_max_x = text_pos.x + galley.size.x;
        let desired_width = text_max_x + button_padding.x - available.left();
        let desired_width = desired_width.max(available.width());

        let mut desired_size = vec2(desired_width, galley.size.y + 2.0 * button_padding.y);
        desired_size = desired_size.at_least(ui.spacing().interact_size);
        let (_, rect) = ui.allocate_space(desired_size);

        let mut header_response = ui.interact(rect, id, Sense::click());
        let text_pos = pos2(
            text_pos.x,
            header_response.rect.center().y - galley.size.y / 2.0,
        );

        let mut state = State::from_memory_with_default_open(ui.ctx(), id, default_open);
        if header_response.clicked() {
            state.toggle(ui);
            header_response.mark_changed();
        }
        header_response
            .widget_info(|| WidgetInfo::labeled(WidgetType::CollapsingHeader, &galley.text));

        let visuals = ui.style().interact(&header_response);
        let text_color = visuals.text_color();
        ui.painter().add(Shape::Rect {
            rect: header_response.rect.expand(visuals.expansion),
            corner_radius: visuals.corner_radius,
            fill: visuals.bg_fill,
            stroke: visuals.bg_stroke,
            // stroke: Default::default(),
        });

        {
            let (mut icon_rect, _) = ui.spacing().icon_rectangles(header_response.rect);
            icon_rect.set_center(pos2(
                header_response.rect.left() + ui.spacing().indent / 2.0,
                header_response.rect.center().y,
            ));
            let icon_response = Response {
                rect: icon_rect,
                ..header_response.clone()
            };
            let openness = state.openness(ui.ctx(), id);
            paint_icon(ui, openness, &icon_response);
        }

        ui.painter().galley(text_pos, galley, text_color);

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
        // Make sure contents are bellow header,
        // and make sure it is one unit (necessary for putting a `CollapsingHeader` in a grid).
        ui.vertical(|ui| {
            ui.set_enabled(self.enabled);

            let Prepared {
                id,
                header_response,
                mut state,
            } = self.begin(ui);

            let ret_response = state.add_contents(ui, id, |ui| {
                ui.indent(id, |ui| {
                    // make as wide as the header:
                    ui.expand_to_include_x(header_response.rect.right());
                    add_contents(ui)
                })
                .inner
            });
            ui.memory().id_data.insert(id, state);

            if let Some(ret_response) = ret_response {
                CollapsingResponse {
                    header_response,
                    body_response: Some(ret_response.response),
                    body_returned: Some(ret_response.inner),
                }
            } else {
                CollapsingResponse {
                    header_response,
                    body_response: None,
                    body_returned: None,
                }
            }
        })
        .inner
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
