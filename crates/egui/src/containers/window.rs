// WARNING: the code in here is horrible. It is a behemoth that needs breaking up into simpler parts.

use std::sync::Arc;

use crate::collapsing_header::CollapsingState;
use crate::*;
use epaint::*;

use super::*;

/// Builder for a floating window which can be dragged, closed, collapsed, resized and scrolled (off by default).
///
/// You can customize:
/// * title
/// * default, minimum, maximum and/or fixed size, collapsed/expanded
/// * if the window has a scroll area (off by default)
/// * if the window can be collapsed (minimized) to just the title bar (yes, by default)
/// * if there should be a close button (none by default)
///
/// ```
/// # egui::__run_test_ctx(|ctx| {
/// egui::Window::new("My Window").show(ctx, |ui| {
///    ui.label("Hello World!");
/// });
/// # });
/// ```
///
/// The previous rectangle used by this window can be obtained through [`crate::Memory::area_rect()`].
///
/// Note that this is NOT a native OS window.
/// To create a new native OS window, use [`crate::Context::show_viewport_deferred`].
#[must_use = "You should call .show()"]
pub struct Window<'open> {
    title: WidgetText,
    open: Option<&'open mut bool>,
    area: Area,
    frame: Option<Frame>,
    resize: Resize,
    scroll: ScrollArea,
    collapsible: bool,
    default_open: bool,
    with_title_bar: bool,
}

impl<'open> Window<'open> {
    /// The window title is used as a unique [`Id`] and must be unique, and should not change.
    /// This is true even if you disable the title bar with `.title_bar(false)`.
    /// If you need a changing title, you must call `window.id(…)` with a fixed id.
    pub fn new(title: impl Into<WidgetText>) -> Self {
        let title = title.into().fallback_text_style(TextStyle::Heading);
        let area = Area::new(Id::new(title.text())).constrain(true);
        Self {
            title,
            open: None,
            area,
            frame: None,
            resize: Resize::default()
                .with_stroke(false)
                .min_size([96.0, 32.0])
                .default_size([340.0, 420.0]), // Default inner size of a window
            scroll: ScrollArea::neither(),
            collapsible: true,
            default_open: true,
            with_title_bar: true,
        }
    }

    /// Assign a unique id to the Window. Required if the title changes, or is shared with another window.
    #[inline]
    pub fn id(mut self, id: Id) -> Self {
        self.area = self.area.id(id);
        self
    }

    /// Call this to add a close-button to the window title bar.
    ///
    /// * If `*open == false`, the window will not be visible.
    /// * If `*open == true`, the window will have a close button.
    /// * If the close button is pressed, `*open` will be set to `false`.
    #[inline]
    pub fn open(mut self, open: &'open mut bool) -> Self {
        self.open = Some(open);
        self
    }

    /// If `false` the window will be grayed out and non-interactive.
    #[inline]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.area = self.area.enabled(enabled);
        self
    }

    /// If `false` the window will be non-interactive.
    #[inline]
    pub fn interactable(mut self, interactable: bool) -> Self {
        self.area = self.area.interactable(interactable);
        self
    }

    /// If `false` the window will be immovable.
    #[inline]
    pub fn movable(mut self, movable: bool) -> Self {
        self.area = self.area.movable(movable);
        self
    }

    /// Usage: `Window::new(…).mutate(|w| w.resize = w.resize.auto_expand_width(true))`
    // TODO(emilk): I'm not sure this is a good interface for this.
    #[inline]
    pub fn mutate(mut self, mutate: impl Fn(&mut Self)) -> Self {
        mutate(&mut self);
        self
    }

    /// Usage: `Window::new(…).resize(|r| r.auto_expand_width(true))`
    // TODO(emilk): I'm not sure this is a good interface for this.
    #[inline]
    pub fn resize(mut self, mutate: impl Fn(Resize) -> Resize) -> Self {
        self.resize = mutate(self.resize);
        self
    }

    /// Change the background color, margins, etc.
    #[inline]
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }

    /// Set minimum width of the window.
    #[inline]
    pub fn min_width(mut self, min_width: f32) -> Self {
        self.resize = self.resize.min_width(min_width);
        self
    }

    /// Set minimum height of the window.
    #[inline]
    pub fn min_height(mut self, min_height: f32) -> Self {
        self.resize = self.resize.min_height(min_height);
        self
    }

    /// Set minimum size of the window, equivalent to calling both `min_width` and `min_height`.
    #[inline]
    pub fn min_size(mut self, min_size: impl Into<Vec2>) -> Self {
        self.resize = self.resize.min_size(min_size);
        self
    }

    /// Set maximum width of the window.
    #[inline]
    pub fn max_width(mut self, max_width: f32) -> Self {
        self.resize = self.resize.max_width(max_width);
        self
    }

    /// Set maximum height of the window.
    #[inline]
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.resize = self.resize.max_height(max_height);
        self
    }

    /// Set maximum size of the window, equivalent to calling both `max_width` and `max_height`.
    #[inline]
    pub fn max_size(mut self, max_size: impl Into<Vec2>) -> Self {
        self.resize = self.resize.max_size(max_size);
        self
    }

    /// Set current position of the window.
    /// If the window is movable it is up to you to keep track of where it moved to!
    #[inline]
    pub fn current_pos(mut self, current_pos: impl Into<Pos2>) -> Self {
        self.area = self.area.current_pos(current_pos);
        self
    }

    /// Set initial position of the window.
    #[inline]
    pub fn default_pos(mut self, default_pos: impl Into<Pos2>) -> Self {
        self.area = self.area.default_pos(default_pos);
        self
    }

    /// Sets the window position and prevents it from being dragged around.
    #[inline]
    pub fn fixed_pos(mut self, pos: impl Into<Pos2>) -> Self {
        self.area = self.area.fixed_pos(pos);
        self
    }

    /// Constrains this window to the screen bounds.
    ///
    /// To change the area to constrain to, use [`Self::constrain_to`].
    ///
    /// Default: `true`.
    #[inline]
    pub fn constrain(mut self, constrain: bool) -> Self {
        self.area = self.area.constrain(constrain);
        self
    }

    /// Constrain the movement of the window to the given rectangle.
    ///
    /// For instance: `.constrain_to(ctx.screen_rect())`.
    #[inline]
    pub fn constrain_to(mut self, constrain_rect: Rect) -> Self {
        self.area = self.area.constrain_to(constrain_rect);
        self
    }

    /// Where the "root" of the window is.
    ///
    /// For instance, if you set this to [`Align2::RIGHT_TOP`]
    /// then [`Self::fixed_pos`] will set the position of the right-top
    /// corner of the window.
    ///
    /// Default: [`Align2::LEFT_TOP`].
    #[inline]
    pub fn pivot(mut self, pivot: Align2) -> Self {
        self.area = self.area.pivot(pivot);
        self
    }

    /// Set anchor and distance.
    ///
    /// An anchor of `Align2::RIGHT_TOP` means "put the right-top corner of the window
    /// in the right-top corner of the screen".
    ///
    /// The offset is added to the position, so e.g. an offset of `[-5.0, 5.0]`
    /// would move the window left and down from the given anchor.
    ///
    /// Anchoring also makes the window immovable.
    ///
    /// It is an error to set both an anchor and a position.
    #[inline]
    pub fn anchor(mut self, align: Align2, offset: impl Into<Vec2>) -> Self {
        self.area = self.area.anchor(align, offset);
        self
    }

    /// Set initial collapsed state of the window
    #[inline]
    pub fn default_open(mut self, default_open: bool) -> Self {
        self.default_open = default_open;
        self
    }

    /// Set initial size of the window.
    #[inline]
    pub fn default_size(mut self, default_size: impl Into<Vec2>) -> Self {
        self.resize = self.resize.default_size(default_size);
        self
    }

    /// Set initial width of the window.
    #[inline]
    pub fn default_width(mut self, default_width: f32) -> Self {
        self.resize = self.resize.default_width(default_width);
        self
    }

    /// Set initial height of the window.
    #[inline]
    pub fn default_height(mut self, default_height: f32) -> Self {
        self.resize = self.resize.default_height(default_height);
        self
    }

    /// Sets the window size and prevents it from being resized by dragging its edges.
    #[inline]
    pub fn fixed_size(mut self, size: impl Into<Vec2>) -> Self {
        self.resize = self.resize.fixed_size(size);
        self
    }

    /// Set initial position and size of the window.
    pub fn default_rect(self, rect: Rect) -> Self {
        self.default_pos(rect.min).default_size(rect.size())
    }

    /// Sets the window pos and size and prevents it from being moved and resized by dragging its edges.
    pub fn fixed_rect(self, rect: Rect) -> Self {
        self.fixed_pos(rect.min).fixed_size(rect.size())
    }

    /// Can the user resize the window by dragging its edges?
    ///
    /// Note that even if you set this to `false` the window may still auto-resize.
    ///
    /// You can set the window to only be resizable in one direction by using
    /// e.g. `[true, false]` as the argument,
    /// making the window only resizable in the x-direction.
    ///
    /// Default is `true`.
    #[inline]
    pub fn resizable(mut self, resizable: impl Into<Vec2b>) -> Self {
        let resizable = resizable.into();
        self.resize = self.resize.resizable(resizable);
        self
    }

    /// Can the window be collapsed by clicking on its title?
    #[inline]
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }

    /// Show title bar on top of the window?
    /// If `false`, the window will not be collapsible nor have a close-button.
    #[inline]
    pub fn title_bar(mut self, title_bar: bool) -> Self {
        self.with_title_bar = title_bar;
        self
    }

    /// Not resizable, just takes the size of its contents.
    /// Also disabled scrolling.
    /// Text will not wrap, but will instead make your window width expand.
    #[inline]
    pub fn auto_sized(mut self) -> Self {
        self.resize = self.resize.auto_sized();
        self.scroll = ScrollArea::neither();
        self
    }

    /// Enable/disable horizontal/vertical scrolling. `false` by default.
    #[inline]
    pub fn scroll2(mut self, scroll: impl Into<Vec2b>) -> Self {
        self.scroll = self.scroll.scroll2(scroll);
        self
    }

    /// Enable/disable horizontal scrolling. `false` by default.
    #[inline]
    pub fn hscroll(mut self, hscroll: bool) -> Self {
        self.scroll = self.scroll.hscroll(hscroll);
        self
    }

    /// Enable/disable vertical scrolling. `false` by default.
    #[inline]
    pub fn vscroll(mut self, vscroll: bool) -> Self {
        self.scroll = self.scroll.vscroll(vscroll);
        self
    }

    /// Enable/disable scrolling on the window by dragging with the pointer. `true` by default.
    ///
    /// See [`ScrollArea::drag_to_scroll`] for more.
    #[inline]
    pub fn drag_to_scroll(mut self, drag_to_scroll: bool) -> Self {
        self.scroll = self.scroll.drag_to_scroll(drag_to_scroll);
        self
    }
}

impl<'open> Window<'open> {
    /// Returns `None` if the window is not open (if [`Window::open`] was called with `&mut false`).
    /// Returns `Some(InnerResponse { inner: None })` if the window is collapsed.
    #[inline]
    pub fn show<R>(
        self,
        ctx: &Context,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<Option<R>>> {
        self.show_dyn(ctx, Box::new(add_contents))
    }

    fn show_dyn<'c, R>(
        self,
        ctx: &Context,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> Option<InnerResponse<Option<R>>> {
        let Window {
            title,
            open,
            area,
            frame,
            resize,
            scroll,
            collapsible,
            default_open,
            with_title_bar,
        } = self;

        let header_color =
            frame.map_or_else(|| ctx.style().visuals.widgets.open.weak_bg_fill, |f| f.fill);
        let mut window_frame = frame.unwrap_or_else(|| Frame::window(&ctx.style()));
        // Keep the original inner margin for later use
        let window_margin = window_frame.inner_margin;
        let border_padding = window_frame.stroke.width / 2.0;
        // Add border padding to the inner margin to prevent it from covering the contents
        window_frame.inner_margin += border_padding;

        let is_explicitly_closed = matches!(open, Some(false));
        let is_open = !is_explicitly_closed || ctx.memory(|mem| mem.everything_is_visible());
        area.show_open_close_animation(ctx, &window_frame, is_open);

        if !is_open {
            return None;
        }

        let area_id = area.id;
        let area_layer_id = area.layer();
        let resize_id = area_id.with("resize");
        let mut collapsing =
            CollapsingState::load_with_default_open(ctx, area_id.with("collapsing"), default_open);

        let is_collapsed = with_title_bar && !collapsing.is_open();
        let possible = PossibleInteractions::new(&area, &resize, is_collapsed);

        let resize = resize.resizable(false); // We resize it manually
        let mut resize = resize.id(resize_id);

        let on_top = Some(area_layer_id) == ctx.top_layer_id();
        let mut area = area.begin(ctx);

        // Calculate roughly how much larger the window size is compared to the inner rect
        let (title_bar_height, title_content_spacing) = if with_title_bar {
            let style = ctx.style();
            let spacing = window_margin.top + window_margin.bottom;
            let height = ctx.fonts(|f| title.font_height(f, &style)) + spacing;
            window_frame.rounding.ne = window_frame.rounding.ne.clamp(0.0, height / 2.0);
            window_frame.rounding.nw = window_frame.rounding.nw.clamp(0.0, height / 2.0);
            (height, spacing)
        } else {
            (0.0, 0.0)
        };

        {
            // Prevent window from becoming larger than the constraint rect and/or screen rect.
            let screen_rect = ctx.screen_rect();
            let max_rect = area.constrain_rect().unwrap_or(screen_rect);
            let max_width = max_rect.width();
            let max_height = max_rect.height() - title_bar_height;
            resize.max_size.x = resize.max_size.x.min(max_width);
            resize.max_size.y = resize.max_size.y.min(max_height);
        }

        // First check for resize to avoid frame delay:
        let last_frame_outer_rect = area.state().rect();
        let resize_interaction =
            resize_interaction(ctx, possible, area_layer_id, last_frame_outer_rect);

        let margins = window_frame.outer_margin.sum()
            + window_frame.inner_margin.sum()
            + vec2(0.0, title_bar_height);

        resize_response(
            resize_interaction,
            ctx,
            margins,
            area_layer_id,
            &mut area,
            resize_id,
        );

        let mut area_content_ui = area.content_ui(ctx);

        let content_inner = {
            // BEGIN FRAME --------------------------------
            let frame_stroke = window_frame.stroke;
            let mut frame = window_frame.begin(&mut area_content_ui);

            let show_close_button = open.is_some();

            let where_to_put_header_background = &area_content_ui.painter().add(Shape::Noop);

            // Backup item spacing before the title bar
            let item_spacing = frame.content_ui.spacing().item_spacing;
            // Use title bar spacing as the item spacing before the content
            frame.content_ui.spacing_mut().item_spacing.y = title_content_spacing;

            let title_bar = if with_title_bar {
                let title_bar = show_title_bar(
                    &mut frame.content_ui,
                    title,
                    show_close_button,
                    &mut collapsing,
                    collapsible,
                );
                resize.min_size.x = resize.min_size.x.at_least(title_bar.rect.width()); // Prevent making window smaller than title bar width
                Some(title_bar)
            } else {
                None
            };

            // Remove item spacing after the title bar
            frame.content_ui.spacing_mut().item_spacing.y = 0.0;

            let (content_inner, mut content_response) = collapsing
                .show_body_unindented(&mut frame.content_ui, |ui| {
                    // Restore item spacing for the content
                    ui.spacing_mut().item_spacing.y = item_spacing.y;

                    resize.show(ui, |ui| {
                        if scroll.is_any_scroll_enabled() {
                            scroll.show(ui, add_contents).inner
                        } else {
                            add_contents(ui)
                        }
                    })
                })
                .map_or((None, None), |ir| (Some(ir.inner), Some(ir.response)));

            let outer_rect = frame.end(&mut area_content_ui).rect;
            paint_resize_corner(
                &area_content_ui,
                &possible,
                outer_rect,
                frame_stroke,
                window_frame.rounding,
            );

            // END FRAME --------------------------------

            if let Some(title_bar) = title_bar {
                let mut title_rect = Rect::from_min_size(
                    outer_rect.min + vec2(border_padding, border_padding),
                    Vec2 {
                        x: outer_rect.size().x - border_padding * 2.0,
                        y: title_bar_height,
                    },
                );

                title_rect = area_content_ui.painter().round_rect_to_pixels(title_rect);

                if on_top && area_content_ui.visuals().window_highlight_topmost {
                    let mut round = window_frame.rounding;

                    // Eliminate the rounding gap between the title bar and the window frame
                    round -= border_padding;

                    if !is_collapsed {
                        round.se = 0.0;
                        round.sw = 0.0;
                    }

                    area_content_ui.painter().set(
                        *where_to_put_header_background,
                        RectShape::filled(title_rect, round, header_color),
                    );
                };

                // Fix title bar separator line position
                if let Some(response) = &mut content_response {
                    response.rect.min.y = outer_rect.min.y + title_bar_height + border_padding;
                }

                title_bar.ui(
                    &mut area_content_ui,
                    title_rect,
                    &content_response,
                    open,
                    &mut collapsing,
                    collapsible,
                );
            }

            collapsing.store(ctx);

            paint_frame_interaction(&area_content_ui, outer_rect, resize_interaction);

            content_inner
        };

        let full_response = area.end(ctx, area_content_ui);

        let inner_response = InnerResponse {
            inner: content_inner,
            response: full_response,
        };
        Some(inner_response)
    }
}

fn paint_resize_corner(
    ui: &Ui,
    possible: &PossibleInteractions,
    outer_rect: Rect,
    stroke: impl Into<Stroke>,
    rounding: impl Into<Rounding>,
) {
    let stroke = stroke.into();
    let rounding = rounding.into();
    let (corner, radius) = if possible.resize_right && possible.resize_bottom {
        (Align2::RIGHT_BOTTOM, rounding.se)
    } else if possible.resize_left && possible.resize_bottom {
        (Align2::LEFT_BOTTOM, rounding.sw)
    } else if possible.resize_left && possible.resize_top {
        (Align2::LEFT_TOP, rounding.nw)
    } else if possible.resize_right && possible.resize_top {
        (Align2::RIGHT_TOP, rounding.ne)
    } else {
        // We're not in two directions, but it is still nice to tell the user
        // we're resizable by painting the resize corner in the expected place
        // (i.e. for windows only resizable in one direction):
        if possible.resize_right || possible.resize_bottom {
            (Align2::RIGHT_BOTTOM, rounding.se)
        } else if possible.resize_left || possible.resize_bottom {
            (Align2::LEFT_BOTTOM, rounding.sw)
        } else if possible.resize_left || possible.resize_top {
            (Align2::LEFT_TOP, rounding.nw)
        } else if possible.resize_right || possible.resize_top {
            (Align2::RIGHT_TOP, rounding.ne)
        } else {
            return;
        }
    };

    // Adjust the corner offset to accommodate the stroke width and window rounding
    let offset = if radius <= 2.0 && stroke.width < 2.0 {
        2.0
    } else {
        // The corner offset is calculated to make the corner appear to be in the correct position
        (2.0_f32.sqrt() * (1.0 + radius + stroke.width / 2.0) - radius)
            * 45.0_f32.to_radians().cos()
    };
    let corner_size = Vec2::splat(ui.visuals().resize_corner_size);
    let corner_rect = corner.align_size_within_rect(corner_size, outer_rect);
    let corner_rect = corner_rect.translate(-offset * corner.to_sign()); // move away from corner
    crate::resize::paint_resize_corner_with_style(ui, &corner_rect, stroke.color, corner);
}

// ----------------------------------------------------------------------------

/// Which sides can be resized?
#[derive(Clone, Copy, Debug)]
struct PossibleInteractions {
    // Which sides can we drag to resize or move?
    resize_left: bool,
    resize_right: bool,
    resize_top: bool,
    resize_bottom: bool,
}

impl PossibleInteractions {
    fn new(area: &Area, resize: &Resize, is_collapsed: bool) -> Self {
        let movable = area.is_enabled() && area.is_movable();
        let resizable = resize
            .is_resizable()
            .and(area.is_enabled() && !is_collapsed);
        let pivot = area.get_pivot();
        Self {
            resize_left: resizable.x && (movable || pivot.x() != Align::LEFT),
            resize_right: resizable.x && (movable || pivot.x() != Align::RIGHT),
            resize_top: resizable.y && (movable || pivot.y() != Align::TOP),
            resize_bottom: resizable.y && (movable || pivot.y() != Align::BOTTOM),
        }
    }

    pub fn resizable(&self) -> bool {
        self.resize_left || self.resize_right || self.resize_top || self.resize_bottom
    }
}

/// Resizing the window edges.
#[derive(Clone, Copy, Debug)]
struct ResizeInteraction {
    start_rect: Rect,
    left: SideResponse,
    right: SideResponse,
    top: SideResponse,
    bottom: SideResponse,
}

/// A minitature version of `Response`, for each side of the window.
#[derive(Clone, Copy, Debug, Default)]
struct SideResponse {
    hover: bool,
    drag: bool,
}

impl SideResponse {
    pub fn any(&self) -> bool {
        self.hover || self.drag
    }
}

impl std::ops::BitOrAssign for SideResponse {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = Self {
            hover: self.hover || rhs.hover,
            drag: self.drag || rhs.drag,
        };
    }
}

impl ResizeInteraction {
    pub fn set_cursor(&self, ctx: &Context) {
        let left = self.left.any();
        let right = self.right.any();
        let top = self.top.any();
        let bottom = self.bottom.any();

        if (left && top) || (right && bottom) {
            ctx.set_cursor_icon(CursorIcon::ResizeNwSe);
        } else if (right && top) || (left && bottom) {
            ctx.set_cursor_icon(CursorIcon::ResizeNeSw);
        } else if left || right {
            ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
        } else if bottom || top {
            ctx.set_cursor_icon(CursorIcon::ResizeVertical);
        }
    }

    pub fn any_hovered(&self) -> bool {
        self.left.hover || self.right.hover || self.top.hover || self.bottom.hover
    }

    pub fn any_dragged(&self) -> bool {
        self.left.drag || self.right.drag || self.top.drag || self.bottom.drag
    }
}

fn resize_response(
    resize_interaction: ResizeInteraction,
    ctx: &Context,
    margins: Vec2,
    area_layer_id: LayerId,
    area: &mut area::Prepared,
    resize_id: Id,
) {
    let Some(new_rect) = move_and_resize_window(ctx, &resize_interaction) else {
        return;
    };
    let mut new_rect = ctx.round_rect_to_pixels(new_rect);

    if area.constrain() {
        new_rect = ctx.constrain_window_rect_to_area(new_rect, area.constrain_rect());
    }

    // TODO(emilk): add this to a Window state instead as a command "move here next frame"
    area.state_mut().set_left_top_pos(new_rect.left_top());

    if resize_interaction.any_dragged() {
        if let Some(mut state) = resize::State::load(ctx, resize_id) {
            state.requested_size = Some(new_rect.size() - margins);
            state.store(ctx, resize_id);
        }
    }

    ctx.memory_mut(|mem| mem.areas_mut().move_to_top(area_layer_id));
}

fn move_and_resize_window(ctx: &Context, interaction: &ResizeInteraction) -> Option<Rect> {
    if !interaction.any_dragged() {
        return None;
    }

    let pointer_pos = ctx.input(|i| i.pointer.interact_pos())?;
    let mut rect = interaction.start_rect; // prevent drift

    if interaction.left.drag {
        rect.min.x = ctx.round_to_pixel(pointer_pos.x);
    } else if interaction.right.drag {
        rect.max.x = ctx.round_to_pixel(pointer_pos.x);
    }

    if interaction.top.drag {
        rect.min.y = ctx.round_to_pixel(pointer_pos.y);
    } else if interaction.bottom.drag {
        rect.max.y = ctx.round_to_pixel(pointer_pos.y);
    }

    Some(rect)
}

fn resize_interaction(
    ctx: &Context,
    possible: PossibleInteractions,
    layer_id: LayerId,
    rect: Rect,
) -> ResizeInteraction {
    if !possible.resizable() {
        return ResizeInteraction {
            start_rect: rect,
            left: Default::default(),
            right: Default::default(),
            top: Default::default(),
            bottom: Default::default(),
        };
    }

    let is_dragging = |rect, id| {
        let response = ctx.create_widget(WidgetRect {
            layer_id,
            id,
            rect,
            interact_rect: rect,
            sense: Sense::drag(),
            enabled: true,
        });
        SideResponse {
            hover: response.hovered(),
            drag: response.dragged(),
        }
    };

    let id = Id::new(layer_id).with("edge_drag");

    let side_grab_radius = ctx.style().interaction.resize_grab_radius_side;
    let corner_grab_radius = ctx.style().interaction.resize_grab_radius_corner;

    let corner_rect =
        |center: Pos2| Rect::from_center_size(center, Vec2::splat(2.0 * corner_grab_radius));

    // What are we dragging/hovering?
    let [mut left, mut right, mut top, mut bottom] = [SideResponse::default(); 4];

    // ----------------------------------------
    // Check sides first, so that corners are on top, covering the sides (i.e. corners have priority)

    if possible.resize_right {
        let response = is_dragging(
            Rect::from_min_max(rect.right_top(), rect.right_bottom()).expand(side_grab_radius),
            id.with("right"),
        );
        right |= response;
    }
    if possible.resize_left {
        let response = is_dragging(
            Rect::from_min_max(rect.left_top(), rect.left_bottom()).expand(side_grab_radius),
            id.with("left"),
        );
        left |= response;
    }
    if possible.resize_bottom {
        let response = is_dragging(
            Rect::from_min_max(rect.left_bottom(), rect.right_bottom()).expand(side_grab_radius),
            id.with("bottom"),
        );
        bottom |= response;
    }
    if possible.resize_top {
        let response = is_dragging(
            Rect::from_min_max(rect.left_top(), rect.right_top()).expand(side_grab_radius),
            id.with("top"),
        );
        top |= response;
    }

    // ----------------------------------------
    // Now check corners:

    if possible.resize_right && possible.resize_bottom {
        let response = is_dragging(corner_rect(rect.right_bottom()), id.with("right_bottom"));
        right |= response;
        bottom |= response;
    }

    if possible.resize_right && possible.resize_top {
        let response = is_dragging(corner_rect(rect.right_top()), id.with("right_top"));
        right |= response;
        top |= response;
    }

    if possible.resize_left && possible.resize_bottom {
        let response = is_dragging(corner_rect(rect.left_bottom()), id.with("left_bottom"));
        left |= response;
        bottom |= response;
    }

    if possible.resize_left && possible.resize_top {
        let response = is_dragging(corner_rect(rect.left_top()), id.with("left_top"));
        left |= response;
        top |= response;
    }

    let interaction = ResizeInteraction {
        start_rect: rect,
        left,
        right,
        top,
        bottom,
    };
    interaction.set_cursor(ctx);
    interaction
}

/// Fill in parts of the window frame when we resize by dragging that part
fn paint_frame_interaction(ui: &Ui, rect: Rect, interaction: ResizeInteraction) {
    use epaint::tessellator::path::add_circle_quadrant;

    let visuals = if interaction.any_dragged() {
        ui.style().visuals.widgets.active
    } else if interaction.any_hovered() {
        ui.style().visuals.widgets.hovered
    } else {
        return;
    };

    let [left, right, top, bottom]: [bool; 4];

    if interaction.any_dragged() {
        left = interaction.left.drag;
        right = interaction.right.drag;
        top = interaction.top.drag;
        bottom = interaction.bottom.drag;
    } else {
        left = interaction.left.hover;
        right = interaction.right.hover;
        top = interaction.top.hover;
        bottom = interaction.bottom.hover;
    }

    let rounding = ui.visuals().window_rounding;
    let Rect { min, max } = rect;

    let mut points = Vec::new();

    if right && !bottom && !top {
        points.push(pos2(max.x, min.y + rounding.ne));
        points.push(pos2(max.x, max.y - rounding.se));
    }
    if right && bottom {
        points.push(pos2(max.x, min.y + rounding.ne));
        points.push(pos2(max.x, max.y - rounding.se));
        add_circle_quadrant(
            &mut points,
            pos2(max.x - rounding.se, max.y - rounding.se),
            rounding.se,
            0.0,
        );
    }
    if bottom {
        points.push(pos2(max.x - rounding.se, max.y));
        points.push(pos2(min.x + rounding.sw, max.y));
    }
    if left && bottom {
        add_circle_quadrant(
            &mut points,
            pos2(min.x + rounding.sw, max.y - rounding.sw),
            rounding.sw,
            1.0,
        );
    }
    if left {
        points.push(pos2(min.x, max.y - rounding.sw));
        points.push(pos2(min.x, min.y + rounding.nw));
    }
    if left && top {
        add_circle_quadrant(
            &mut points,
            pos2(min.x + rounding.nw, min.y + rounding.nw),
            rounding.nw,
            2.0,
        );
    }
    if top {
        points.push(pos2(min.x + rounding.nw, min.y));
        points.push(pos2(max.x - rounding.ne, min.y));
    }
    if right && top {
        add_circle_quadrant(
            &mut points,
            pos2(max.x - rounding.ne, min.y + rounding.ne),
            rounding.ne,
            3.0,
        );
        points.push(pos2(max.x, min.y + rounding.ne));
        points.push(pos2(max.x, max.y - rounding.se));
    }
    ui.painter().add(Shape::line(points, visuals.bg_stroke));
}

// ----------------------------------------------------------------------------

struct TitleBar {
    /// A title Id used for dragging windows
    id: Id,

    /// Prepared text in the title
    title_galley: Arc<Galley>,

    /// Size of the title bar in a collapsed state (if window is collapsible),
    /// which includes all necessary space for showing the expand button, the
    /// title and the close button.
    min_rect: Rect,

    /// Size of the title bar in an expanded state. This size become known only
    /// after expanding window and painting its content
    rect: Rect,
}

fn show_title_bar(
    ui: &mut Ui,
    title: WidgetText,
    show_close_button: bool,
    collapsing: &mut CollapsingState,
    collapsible: bool,
) -> TitleBar {
    let inner_response = ui.horizontal(|ui| {
        let height = ui
            .fonts(|fonts| title.font_height(fonts, ui.style()))
            .max(ui.spacing().interact_size.y);
        ui.set_min_height(height);

        let item_spacing = ui.spacing().item_spacing;
        let button_size = Vec2::splat(ui.spacing().icon_width);

        let pad = (height - button_size.y) / 2.0; // calculated so that the icon is on the diagonal (if window padding is symmetrical)

        if collapsible {
            ui.add_space(pad);
            collapsing.show_default_button_with_size(ui, button_size);
        }

        let title_galley = title.into_galley(ui, Some(false), f32::INFINITY, TextStyle::Heading);

        let minimum_width = if collapsible || show_close_button {
            // If at least one button is shown we make room for both buttons (since title is centered):
            2.0 * (pad + button_size.x + item_spacing.x) + title_galley.size().x
        } else {
            pad + title_galley.size().x + pad
        };
        let min_rect = Rect::from_min_size(ui.min_rect().min, vec2(minimum_width, height));
        let id = ui.advance_cursor_after_rect(min_rect);

        TitleBar {
            id,
            title_galley,
            min_rect,
            rect: Rect::NAN, // Will be filled in later
        }
    });

    let title_bar = inner_response.inner;
    let rect = inner_response.response.rect;

    TitleBar { rect, ..title_bar }
}

impl TitleBar {
    /// Finishes painting of the title bar when the window content size already known.
    ///
    /// # Parameters
    ///
    /// - `ui`:
    /// - `outer_rect`:
    /// - `content_response`: if `None`, window is collapsed at this frame, otherwise contains
    ///   a result of rendering the window content
    /// - `open`: if `None`, no "Close" button will be rendered, otherwise renders and processes
    ///   the "Close" button and writes a `false` if window was closed
    /// - `collapsing`: holds the current expanding state. Can be changed by double click on the
    ///   title if `collapsible` is `true`
    /// - `collapsible`: if `true`, double click on the title bar will be handled for a change
    ///   of `collapsing` state
    fn ui(
        mut self,
        ui: &mut Ui,
        outer_rect: Rect,
        content_response: &Option<Response>,
        open: Option<&mut bool>,
        collapsing: &mut CollapsingState,
        collapsible: bool,
    ) {
        if let Some(content_response) = &content_response {
            // Now we know how large we got to be:
            self.rect.max.x = self.rect.max.x.max(content_response.rect.max.x);
        }

        if let Some(open) = open {
            // Add close button now that we know our full width:
            if self.close_button_ui(ui).clicked() {
                *open = false;
            }
        }

        let full_top_rect = Rect::from_x_y_ranges(self.rect.x_range(), self.min_rect.y_range());
        let text_pos =
            emath::align::center_size_in_rect(self.title_galley.size(), full_top_rect).left_top();
        let text_pos = text_pos - self.title_galley.rect.min.to_vec2();
        let text_pos = text_pos - 1.5 * Vec2::Y; // HACK: center on x-height of text (looks better)
        ui.painter().galley(
            text_pos,
            self.title_galley.clone(),
            ui.visuals().text_color(),
        );

        if let Some(content_response) = &content_response {
            // paint separator between title and content:
            let y = content_response.rect.top();
            // let y = lerp(self.rect.bottom()..=content_response.rect.top(), 0.5);
            let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
            // Workaround: To prevent border infringement,
            // the 0.1 value should ideally be calculated using TessellationOptions::feathering_size_in_pixels
            let x_range = outer_rect.x_range().shrink(0.1);
            ui.painter().hline(x_range, y, stroke);
        }

        // Don't cover the close- and collapse buttons:
        let double_click_rect = self.rect.shrink2(vec2(32.0, 0.0));

        if ui
            .interact(double_click_rect, self.id, Sense::click())
            .double_clicked()
            && collapsible
        {
            collapsing.toggle(ui);
        }
    }

    /// Paints the "Close" button at the right side of the title bar
    /// and processes clicks on it.
    ///
    /// The button is square and its size is determined by the
    /// [`crate::style::Spacing::icon_width`] setting.
    fn close_button_ui(&self, ui: &mut Ui) -> Response {
        let button_size = Vec2::splat(ui.spacing().icon_width);
        let pad = (self.rect.height() - button_size.y) / 2.0; // calculated so that the icon is on the diagonal (if window padding is symmetrical)
        let button_rect = Rect::from_min_size(
            pos2(
                self.rect.right() - pad - button_size.x,
                self.rect.center().y - 0.5 * button_size.y,
            ),
            button_size,
        );

        close_button(ui, button_rect)
    }
}

/// Paints the "Close" button of the window and processes clicks on it.
///
/// The close button is just an `X` symbol painted by a current stroke
/// for foreground elements (such as a label text).
///
/// # Parameters
/// - `ui`:
/// - `rect`: The rectangular area to fit the button in
///
/// Returns the result of a click on a button if it was pressed
fn close_button(ui: &mut Ui, rect: Rect) -> Response {
    let close_id = ui.auto_id_with("window_close_button");
    let response = ui.interact(rect, close_id, Sense::click());
    ui.expand_to_include_rect(response.rect);

    let visuals = ui.style().interact(&response);
    let rect = rect.shrink(2.0).expand(visuals.expansion);
    let stroke = visuals.fg_stroke;
    ui.painter() // paints \
        .line_segment([rect.left_top(), rect.right_bottom()], stroke);
    ui.painter() // paints /
        .line_segment([rect.right_top(), rect.left_bottom()], stroke);
    response
}
