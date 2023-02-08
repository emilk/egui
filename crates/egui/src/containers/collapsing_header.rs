use std::hash::Hash;

use crate::*;
use epaint::Shape;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) struct InnerState {
    open: bool,

    /// Height of the region when open. Used for animations
    #[cfg_attr(feature = "serde", serde(default))]
    open_height: Option<f32>,
}

/// This is a a building block for building collapsing regions.
///
/// It is used by [`CollapsingHeader`] and [`Window`], but can also be used on its own.
///
/// See [`CollapsingState::show_header`] for how to show a collapsing header with a custom header.
#[derive(Clone, Debug)]
pub struct CollapsingState {
    id: Id,
    state: InnerState,
}

impl CollapsingState {
    pub fn load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data_mut(|d| {
            d.get_persisted::<InnerState>(id)
                .map(|state| Self { id, state })
        })
    }

    pub fn store(&self, ctx: &Context) {
        ctx.data_mut(|d| d.insert_persisted(self.id, self.state));
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn load_with_default_open(ctx: &Context, id: Id, default_open: bool) -> Self {
        Self::load(ctx, id).unwrap_or(CollapsingState {
            id,
            state: InnerState {
                open: default_open,
                open_height: None,
            },
        })
    }

    pub fn is_open(&self) -> bool {
        self.state.open
    }

    pub fn set_open(&mut self, open: bool) {
        self.state.open = open;
    }

    pub fn toggle(&mut self, ui: &Ui) {
        self.state.open = !self.state.open;
        ui.ctx().request_repaint();
    }

    /// 0 for closed, 1 for open, with tweening
    pub fn openness(&self, ctx: &Context) -> f32 {
        if ctx.memory(|mem| mem.everything_is_visible()) {
            1.0
        } else {
            ctx.animate_bool(self.id, self.state.open)
        }
    }

    /// Will toggle when clicked, etc.
    pub(crate) fn show_default_button_with_size(
        &mut self,
        ui: &mut Ui,
        button_size: Vec2,
    ) -> Response {
        let (_id, rect) = ui.allocate_space(button_size);
        let response = ui.interact(rect, self.id, Sense::click());
        if response.clicked() {
            self.toggle(ui);
        }
        let openness = self.openness(ui.ctx());
        paint_default_icon(ui, openness, &response);
        response
    }

    /// Will toggle when clicked, etc.
    fn show_default_button_indented(&mut self, ui: &mut Ui) -> Response {
        self.show_button_indented(ui, paint_default_icon)
    }

    /// Will toggle when clicked, etc.
    fn show_button_indented(
        &mut self,
        ui: &mut Ui,
        icon_fn: impl FnOnce(&mut Ui, f32, &Response) + 'static,
    ) -> Response {
        let size = vec2(ui.spacing().indent, ui.spacing().icon_width);
        let (_id, rect) = ui.allocate_space(size);
        let response = ui.interact(rect, self.id, Sense::click());
        if response.clicked() {
            self.toggle(ui);
        }

        let (mut icon_rect, _) = ui.spacing().icon_rectangles(response.rect);
        icon_rect.set_center(pos2(
            response.rect.left() + ui.spacing().indent / 2.0,
            response.rect.center().y,
        ));
        let openness = self.openness(ui.ctx());
        let small_icon_response = response.clone().with_new_rect(icon_rect);
        icon_fn(ui, openness, &small_icon_response);
        response
    }

    /// Shows header and body (if expanded).
    ///
    /// The header will start with the default button in a horizontal layout, followed by whatever you add.
    ///
    /// Will also store the state.
    ///
    /// Returns the response of the collapsing button, the custom header, and the custom body.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// let id = ui.make_persistent_id("my_collapsing_header");
    /// egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
    ///     .show_header(ui, |ui| {
    ///         ui.label("Header"); // you can put checkboxes or whatever here
    ///     })
    ///     .body(|ui| ui.label("Body"));
    /// # });
    /// ```
    pub fn show_header<HeaderRet>(
        mut self,
        ui: &mut Ui,
        add_header: impl FnOnce(&mut Ui) -> HeaderRet,
    ) -> HeaderResponse<'_, HeaderRet> {
        let header_response = ui.horizontal(|ui| {
            let prev_item_spacing = ui.spacing_mut().item_spacing;
            ui.spacing_mut().item_spacing.x = 0.0; // the toggler button uses the full indent width
            let collapser = self.show_default_button_indented(ui);
            ui.spacing_mut().item_spacing = prev_item_spacing;
            (collapser, add_header(ui))
        });
        HeaderResponse {
            state: self,
            ui,
            toggle_button_response: header_response.inner.0,
            header_response: InnerResponse {
                response: header_response.response,
                inner: header_response.inner.1,
            },
        }
    }

    /// Show body if we are open, with a nice animation between closed and open.
    /// Indent the body to show it belongs to the header.
    ///
    /// Will also store the state.
    pub fn show_body_indented<R>(
        &mut self,
        header_response: &Response,
        ui: &mut Ui,
        add_body: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        let id = self.id;
        self.show_body_unindented(ui, |ui| {
            ui.indent(id, |ui| {
                // make as wide as the header:
                ui.expand_to_include_x(header_response.rect.right());
                add_body(ui)
            })
            .inner
        })
    }

    /// Show body if we are open, with a nice animation between closed and open.
    /// Will also store the state.
    pub fn show_body_unindented<R>(
        &mut self,
        ui: &mut Ui,
        add_body: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        let openness = self.openness(ui.ctx());
        if openness <= 0.0 {
            self.store(ui.ctx()); // we store any earlier toggling as promised in the docstring
            None
        } else if openness < 1.0 {
            Some(ui.scope(|child_ui| {
                let max_height = if self.state.open && self.state.open_height.is_none() {
                    // First frame of expansion.
                    // We don't know full height yet, but we will next frame.
                    // Just use a placeholder value that shows some movement:
                    10.0
                } else {
                    let full_height = self.state.open_height.unwrap_or_default();
                    remap_clamp(openness, 0.0..=1.0, 0.0..=full_height)
                };

                let mut clip_rect = child_ui.clip_rect();
                clip_rect.max.y = clip_rect.max.y.min(child_ui.max_rect().top() + max_height);
                child_ui.set_clip_rect(clip_rect);

                let ret = add_body(child_ui);

                let mut min_rect = child_ui.min_rect();
                self.state.open_height = Some(min_rect.height());
                self.store(child_ui.ctx()); // remember the height

                // Pretend children took up at most `max_height` space:
                min_rect.max.y = min_rect.max.y.at_most(min_rect.top() + max_height);
                child_ui.force_set_min_rect(min_rect);
                ret
            }))
        } else {
            let ret_response = ui.scope(add_body);
            let full_size = ret_response.response.rect.size();
            self.state.open_height = Some(full_size.y);
            self.store(ui.ctx()); // remember the height
            Some(ret_response)
        }
    }

    /// Paint this [CollapsingState](CollapsingState)'s toggle button. Takes an [IconPainter](IconPainter) as the icon.
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// fn circle_icon(ui: &mut egui::Ui, openness: f32, response: &egui::Response) {
    ///     let stroke = ui.style().interact(&response).fg_stroke;
    ///     let radius = egui::lerp(2.0..=3.0, openness);
    ///     ui.painter().circle_filled(response.rect.center(), radius, stroke.color);
    /// }
    ///
    /// let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
    ///     ui.ctx(),
    ///     ui.make_persistent_id("my_collapsing_state"),
    ///     false,
    /// );
    ///
    /// let header_res = ui.horizontal(|ui| {
    ///     ui.label("Header");
    ///     state.show_toggle_button(ui, circle_icon);
    /// });
    ///
    /// state.show_body_indented(&header_res.response, ui, |ui| ui.label("Body"));
    /// # });
    /// ```
    pub fn show_toggle_button(
        &mut self,
        ui: &mut Ui,
        icon_fn: impl FnOnce(&mut Ui, f32, &Response) + 'static,
    ) -> Response {
        self.show_button_indented(ui, icon_fn)
    }
}

/// From [`CollapsingState::show_header`].
#[must_use = "Remember to show the body"]
pub struct HeaderResponse<'ui, HeaderRet> {
    state: CollapsingState,
    ui: &'ui mut Ui,
    toggle_button_response: Response,
    header_response: InnerResponse<HeaderRet>,
}

impl<'ui, HeaderRet> HeaderResponse<'ui, HeaderRet> {
    /// Returns the response of the collapsing button, the custom header, and the custom body.
    pub fn body<BodyRet>(
        mut self,
        add_body: impl FnOnce(&mut Ui) -> BodyRet,
    ) -> (
        Response,
        InnerResponse<HeaderRet>,
        Option<InnerResponse<BodyRet>>,
    ) {
        let body_response =
            self.state
                .show_body_indented(&self.header_response.response, self.ui, add_body);
        (
            self.toggle_button_response,
            self.header_response,
            body_response,
        )
    }

    /// Returns the response of the collapsing button, the custom header, and the custom body, without indentation.
    pub fn body_unindented<BodyRet>(
        mut self,
        add_body: impl FnOnce(&mut Ui) -> BodyRet,
    ) -> (
        Response,
        InnerResponse<HeaderRet>,
        Option<InnerResponse<BodyRet>>,
    ) {
        let body_response = self.state.show_body_unindented(self.ui, add_body);
        (
            self.toggle_button_response,
            self.header_response,
            body_response,
        )
    }
}

// ----------------------------------------------------------------------------

/// Paint the arrow icon that indicated if the region is open or not
pub fn paint_default_icon(ui: &mut Ui, openness: f32, response: &Response) {
    let visuals = ui.style().interact(response);

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

    ui.painter().add(Shape::convex_polygon(
        points,
        visuals.fg_stroke.color,
        Stroke::NONE,
    ));
}

/// A function that paints an icon indicating if the region is open or not
pub type IconPainter = Box<dyn FnOnce(&mut Ui, f32, &Response)>;

/// A header which can be collapsed/expanded, revealing a contained [`Ui`] region.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// egui::CollapsingHeader::new("Heading")
///     .show(ui, |ui| {
///         ui.label("Body");
///     });
///
/// // Short version:
/// ui.collapsing("Heading", |ui| { ui.label("Body"); });
/// # });
/// ```
///
/// If you want to customize the header contents, see [`CollapsingState::show_header`].
#[must_use = "You should call .show()"]
pub struct CollapsingHeader {
    text: WidgetText,
    default_open: bool,
    open: Option<bool>,
    id_source: Id,
    enabled: bool,
    selectable: bool,
    selected: bool,
    show_background: bool,
    icon: Option<IconPainter>,
}

impl CollapsingHeader {
    /// The [`CollapsingHeader`] starts out collapsed unless you call `default_open`.
    ///
    /// The label is used as an [`Id`] source.
    /// If the label is unique and static this is fine,
    /// but if it changes or there are several [`CollapsingHeader`] with the same title
    /// you need to provide a unique id source with [`Self::id_source`].
    pub fn new(text: impl Into<WidgetText>) -> Self {
        let text = text.into();
        let id_source = Id::new(text.text());
        Self {
            text,
            default_open: false,
            open: None,
            id_source,
            enabled: true,
            selectable: false,
            selected: false,
            show_background: false,
            icon: None,
        }
    }

    /// By default, the [`CollapsingHeader`] is collapsed.
    /// Call `.default_open(true)` to change this.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Calling `.open(Some(true))` will make the collapsing header open this frame (or stay open).
    ///
    /// Calling `.open(Some(false))` will make the collapsing header close this frame (or stay closed).
    ///
    /// Calling `.open(None)` has no effect (default).
    pub fn open(mut self, open: Option<bool>) -> Self {
        self.open = open;
        self
    }

    /// Explicitly set the source of the [`Id`] of this widget, instead of using title label.
    /// This is useful if the title label is dynamic or not unique.
    pub fn id_source(mut self, id_source: impl Hash) -> Self {
        self.id_source = Id::new(id_source);
        self
    }

    /// If you set this to `false`, the [`CollapsingHeader`] will be grayed out and un-clickable.
    ///
    /// This is a convenience for [`Ui::set_enabled`].
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Can the [`CollapsingHeader`] be selected by clicking it? Default: `false`.
    #[deprecated = "Use the more powerful egui::collapsing_header::CollapsingState::show_header"] // Deprecated in 2022-04-28, before egui 0.18
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// If you set this to 'true', the [`CollapsingHeader`] will be shown as selected.
    ///
    /// Example:
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// let mut selected = false;
    /// let response = egui::CollapsingHeader::new("Select and open me")
    ///     .selectable(true)
    ///     .selected(selected)
    ///     .show(ui, |ui| ui.label("Body"));
    /// if response.header_response.clicked() {
    ///     selected = true;
    /// }
    /// # });
    /// ```
    #[deprecated = "Use the more powerful egui::collapsing_header::CollapsingState::show_header"] // Deprecated in 2022-04-28, before egui 0.18
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Should the [`CollapsingHeader`] show a background behind it? Default: `false`.
    ///
    /// To show it behind all [`CollapsingHeader`] you can just use:
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// ui.visuals_mut().collapsing_header_frame = true;
    /// # });
    /// ```
    pub fn show_background(mut self, show_background: bool) -> Self {
        self.show_background = show_background;
        self
    }

    /// Use the provided function to render a different [`CollapsingHeader`] icon.
    /// Defaults to a triangle that animates as the [`CollapsingHeader`] opens and closes.
    ///
    /// For example:
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// fn circle_icon(ui: &mut egui::Ui, openness: f32, response: &egui::Response) {
    ///     let stroke = ui.style().interact(&response).fg_stroke;
    ///     let radius = egui::lerp(2.0..=3.0, openness);
    ///     ui.painter().circle_filled(response.rect.center(), radius, stroke.color);
    /// }
    ///
    /// egui::CollapsingHeader::new("Circles")
    ///   .icon(circle_icon)
    ///   .show(ui, |ui| { ui.label("Hi!"); });
    /// # });
    /// ```
    pub fn icon(mut self, icon_fn: impl FnOnce(&mut Ui, f32, &Response) + 'static) -> Self {
        self.icon = Some(Box::new(icon_fn));
        self
    }
}

struct Prepared {
    header_response: Response,
    state: CollapsingState,
    openness: f32,
}

impl CollapsingHeader {
    fn begin(self, ui: &mut Ui) -> Prepared {
        assert!(
            ui.layout().main_dir().is_vertical(),
            "Horizontal collapsing is unimplemented"
        );
        let Self {
            icon,
            text,
            default_open,
            open,
            id_source,
            enabled: _,
            selectable,
            selected,
            show_background,
        } = self;

        // TODO(emilk): horizontal layout, with icon and text as labels. Insert background behind using Frame.

        let id = ui.make_persistent_id(id_source);
        let button_padding = ui.spacing().button_padding;

        let available = ui.available_rect_before_wrap();
        let text_pos = available.min + vec2(ui.spacing().indent, 0.0);
        let wrap_width = available.right() - text_pos.x;
        let wrap = Some(false);
        let text = text.into_galley(ui, wrap, wrap_width, TextStyle::Button);
        let text_max_x = text_pos.x + text.size().x;

        let mut desired_width = text_max_x + button_padding.x - available.left();
        if ui.visuals().collapsing_header_frame {
            desired_width = desired_width.max(available.width()); // fill full width
        }

        let mut desired_size = vec2(desired_width, text.size().y + 2.0 * button_padding.y);
        desired_size = desired_size.at_least(ui.spacing().interact_size);
        let (_, rect) = ui.allocate_space(desired_size);

        let mut header_response = ui.interact(rect, id, Sense::click());
        let text_pos = pos2(
            text_pos.x,
            header_response.rect.center().y - text.size().y / 2.0,
        );

        let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, default_open);
        if let Some(open) = open {
            if open != state.is_open() {
                state.toggle(ui);
                header_response.mark_changed();
            }
        } else if header_response.clicked() {
            state.toggle(ui);
            header_response.mark_changed();
        }

        header_response
            .widget_info(|| WidgetInfo::labeled(WidgetType::CollapsingHeader, text.text()));

        let openness = state.openness(ui.ctx());

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact_selectable(&header_response, selected);

            if ui.visuals().collapsing_header_frame || show_background {
                ui.painter().add(epaint::RectShape {
                    rect: header_response.rect.expand(visuals.expansion),
                    rounding: visuals.rounding,
                    fill: visuals.weak_bg_fill,
                    stroke: visuals.bg_stroke,
                    // stroke: Default::default(),
                });
            }

            if selected || selectable && (header_response.hovered() || header_response.has_focus())
            {
                let rect = rect.expand(visuals.expansion);

                ui.painter()
                    .rect(rect, visuals.rounding, visuals.bg_fill, visuals.bg_stroke);
            }

            {
                let (mut icon_rect, _) = ui.spacing().icon_rectangles(header_response.rect);
                icon_rect.set_center(pos2(
                    header_response.rect.left() + ui.spacing().indent / 2.0,
                    header_response.rect.center().y,
                ));
                let icon_response = header_response.clone().with_new_rect(icon_rect);
                if let Some(icon) = icon {
                    icon(ui, openness, &icon_response);
                } else {
                    paint_default_icon(ui, openness, &icon_response);
                }
            }

            text.paint_with_visuals(ui.painter(), text_pos, &visuals);
        }

        Prepared {
            header_response,
            state,
            openness,
        }
    }

    #[inline]
    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_body: impl FnOnce(&mut Ui) -> R,
    ) -> CollapsingResponse<R> {
        self.show_dyn(ui, Box::new(add_body), true)
    }

    #[inline]
    pub fn show_unindented<R>(
        self,
        ui: &mut Ui,
        add_body: impl FnOnce(&mut Ui) -> R,
    ) -> CollapsingResponse<R> {
        self.show_dyn(ui, Box::new(add_body), false)
    }

    fn show_dyn<'c, R>(
        self,
        ui: &mut Ui,
        add_body: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
        indented: bool,
    ) -> CollapsingResponse<R> {
        // Make sure body is bellow header,
        // and make sure it is one unit (necessary for putting a [`CollapsingHeader`] in a grid).
        ui.vertical(|ui| {
            ui.set_enabled(self.enabled);

            let Prepared {
                header_response,
                mut state,
                openness,
            } = self.begin(ui); // show the header

            let ret_response = if indented {
                state.show_body_indented(&header_response, ui, add_body)
            } else {
                state.show_body_unindented(ui, add_body)
            };

            if let Some(ret_response) = ret_response {
                CollapsingResponse {
                    header_response,
                    body_response: Some(ret_response.response),
                    body_returned: Some(ret_response.inner),
                    openness,
                }
            } else {
                CollapsingResponse {
                    header_response,
                    body_response: None,
                    body_returned: None,
                    openness,
                }
            }
        })
        .inner
    }
}

/// The response from showing a [`CollapsingHeader`].
pub struct CollapsingResponse<R> {
    /// Response of the actual clickable header.
    pub header_response: Response,

    /// None iff collapsed.
    pub body_response: Option<Response>,

    /// None iff collapsed.
    pub body_returned: Option<R>,

    /// 0.0 if fully closed, 1.0 if fully open, and something in-between while animating.
    pub openness: f32,
}

impl<R> CollapsingResponse<R> {
    /// Was the [`CollapsingHeader`] fully closed (and not being animated)?
    pub fn fully_closed(&self) -> bool {
        self.openness <= 0.0
    }

    /// Was the [`CollapsingHeader`] fully open (and not being animated)?
    pub fn fully_open(&self) -> bool {
        self.openness >= 1.0
    }
}
