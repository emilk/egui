// #![warn(missing_docs)]

use std::hash::Hash;
use std::sync::Arc;

use epaint::mutex::RwLock;

use crate::{
    containers::*, ecolor::*, epaint::text::Fonts, layout::*, menu::MenuState, placer::Placer,
    util::IdTypeMap, widgets::*, *,
};

// ----------------------------------------------------------------------------

/// This is what you use to place widgets.
///
/// Represents a region of the screen with a type of layout (horizontal or vertical).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// ui.add(egui::Label::new("Hello World!"));
/// ui.label("A shorter and more convenient way to add a label.");
/// ui.horizontal(|ui| {
///     ui.label("Add widgets");
///     if ui.button("on the same row!").clicked() {
///         /* … */
///     }
/// });
/// # });
/// ```
pub struct Ui {
    /// ID of this ui.
    /// Generated based on id of parent ui together with
    /// another source of child identity (e.g. window title).
    /// Acts like a namespace for child uis.
    /// Should be unique and persist predictably from one frame to next
    /// so it can be used as a source for storing state (e.g. window position, or if a collapsing header is open).
    id: Id,

    /// This is used to create a unique interact ID for some widgets.
    /// This value is based on where in the hierarchy of widgets this Ui is in,
    /// and the value is increment with each added child widget.
    /// This works as an Id source only as long as new widgets aren't added or removed.
    /// They are therefore only good for Id:s that has no state.
    next_auto_id_source: u64,

    /// Specifies paint layer, clip rectangle and a reference to [`Context`].
    painter: Painter,

    /// The [`Style`] (visuals, spacing, etc) of this ui.
    /// Commonly many [`Ui`]:s share the same [`Style`].
    /// The [`Ui`] implements copy-on-write for this.
    style: Arc<Style>,

    /// Handles the [`Ui`] size and the placement of new widgets.
    placer: Placer,

    /// If false we are unresponsive to input,
    /// and all widgets will assume a gray style.
    enabled: bool,

    /// Indicates whether this Ui belongs to a Menu.
    menu_state: Option<Arc<RwLock<MenuState>>>,
}

impl Ui {
    // ------------------------------------------------------------------------
    // Creation:

    /// Create a new [`Ui`].
    ///
    /// Normally you would not use this directly, but instead use
    /// [`SidePanel`], [`TopBottomPanel`], [`CentralPanel`], [`Window`] or [`Area`].
    pub fn new(ctx: Context, layer_id: LayerId, id: Id, max_rect: Rect, clip_rect: Rect) -> Self {
        let style = ctx.style();
        Ui {
            id,
            next_auto_id_source: id.with("auto").value(),
            painter: Painter::new(ctx, layer_id, clip_rect),
            style,
            placer: Placer::new(max_rect, Layout::default()),
            enabled: true,
            menu_state: None,
        }
    }

    /// Create a new [`Ui`] at a specific region.
    pub fn child_ui(&mut self, max_rect: Rect, layout: Layout) -> Self {
        self.child_ui_with_id_source(max_rect, layout, "child")
    }

    /// Create a new [`Ui`] at a specific region with a specific id.
    pub fn child_ui_with_id_source(
        &mut self,
        max_rect: Rect,
        layout: Layout,
        id_source: impl Hash,
    ) -> Self {
        crate::egui_assert!(!max_rect.any_nan());
        let next_auto_id_source = Id::new(self.next_auto_id_source).with("child").value();
        self.next_auto_id_source = self.next_auto_id_source.wrapping_add(1);
        let menu_state = self.menu_state();
        Ui {
            id: self.id.with(id_source),
            next_auto_id_source,
            painter: self.painter.clone(),
            style: self.style.clone(),
            placer: Placer::new(max_rect, layout),
            enabled: self.enabled,
            menu_state,
        }
    }

    // -------------------------------------------------

    /// A unique identity of this [`Ui`].
    #[inline]
    pub fn id(&self) -> Id {
        self.id
    }

    /// Style options for this [`Ui`] and its children.
    ///
    /// Note that this may be a different [`Style`] than that of [`Context::style`].
    #[inline]
    pub fn style(&self) -> &Arc<Style> {
        &self.style
    }

    /// Mutably borrow internal [`Style`].
    /// Changes apply to this [`Ui`] and its subsequent children.
    ///
    /// To set the style of all [`Ui`]:s, use [`Context::set_style`].
    ///
    /// Example:
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// ui.style_mut().override_text_style = Some(egui::TextStyle::Heading);
    /// # });
    /// ```
    pub fn style_mut(&mut self) -> &mut Style {
        Arc::make_mut(&mut self.style) // clone-on-write
    }

    /// Changes apply to this [`Ui`] and its subsequent children.
    ///
    /// To set the visuals of all [`Ui`]:s, use [`Context::set_visuals`].
    pub fn set_style(&mut self, style: impl Into<Arc<Style>>) {
        self.style = style.into();
    }

    /// Reset to the default style set in [`Context`].
    pub fn reset_style(&mut self) {
        self.style = self.ctx().style();
    }

    /// The current spacing options for this [`Ui`].
    /// Short for `ui.style().spacing`.
    #[inline]
    pub fn spacing(&self) -> &crate::style::Spacing {
        &self.style.spacing
    }

    /// Mutably borrow internal [`Spacing`](crate::style::Spacing).
    /// Changes apply to this [`Ui`] and its subsequent children.
    ///
    /// Example:
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// ui.spacing_mut().item_spacing = egui::vec2(10.0, 2.0);
    /// # });
    /// ```
    pub fn spacing_mut(&mut self) -> &mut crate::style::Spacing {
        &mut self.style_mut().spacing
    }

    /// The current visuals settings of this [`Ui`].
    /// Short for `ui.style().visuals`.
    #[inline]
    pub fn visuals(&self) -> &crate::Visuals {
        &self.style.visuals
    }

    /// Mutably borrow internal `visuals`.
    /// Changes apply to this [`Ui`] and its subsequent children.
    ///
    /// To set the visuals of all [`Ui`]:s, use [`Context::set_visuals`].
    ///
    /// Example:
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// ui.visuals_mut().override_text_color = Some(egui::Color32::RED);
    /// # });
    /// ```
    pub fn visuals_mut(&mut self) -> &mut crate::Visuals {
        &mut self.style_mut().visuals
    }

    /// Get a reference to the parent [`Context`].
    #[inline]
    pub fn ctx(&self) -> &Context {
        self.painter.ctx()
    }

    /// Use this to paint stuff within this [`Ui`].
    #[inline]
    pub fn painter(&self) -> &Painter {
        &self.painter
    }

    /// If `false`, the [`Ui`] does not allow any interaction and
    /// the widgets in it will draw with a gray look.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Calling `set_enabled(false)` will cause the [`Ui`] to deny all future interaction
    /// and all the widgets will draw with a gray look.
    ///
    /// Usually it is more convenient to use [`Self::add_enabled_ui`] or [`Self::add_enabled`].
    ///
    /// Calling `set_enabled(true)` has no effect - it will NOT re-enable the [`Ui`] once disabled.
    ///
    /// ### Example
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut enabled = true;
    /// ui.group(|ui| {
    ///     ui.checkbox(&mut enabled, "Enable subsection");
    ///     ui.set_enabled(enabled);
    ///     if ui.button("Button that is not always clickable").clicked() {
    ///         /* … */
    ///     }
    /// });
    /// # });
    /// ```
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled &= enabled;
        if !self.enabled && self.is_visible() {
            self.painter
                .set_fade_to_color(Some(self.visuals().fade_out_to_color()));
        }
    }

    /// If `false`, any widgets added to the [`Ui`] will be invisible and non-interactive.
    #[inline]
    pub fn is_visible(&self) -> bool {
        self.painter.is_visible()
    }

    #[deprecated = "Renamed is_visible"]
    pub fn visible(&self) -> bool {
        self.painter.is_visible()
    }

    /// Calling `set_visible(false)` will cause all further widgets to be invisible,
    /// yet still allocate space.
    ///
    /// The widgets will not be interactive (`set_visible(false)` implies `set_enabled(false)`).
    ///
    /// Calling `set_visible(true)` has no effect.
    ///
    /// ### Example
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut visible = true;
    /// ui.group(|ui| {
    ///     ui.checkbox(&mut visible, "Show subsection");
    ///     ui.set_visible(visible);
    ///     if ui.button("Button that is not always shown").clicked() {
    ///         /* … */
    ///     }
    /// });
    /// # });
    /// ```
    pub fn set_visible(&mut self, visible: bool) {
        self.set_enabled(visible);
        if !visible {
            self.painter.set_invisible();
        }
    }

    #[inline]
    pub fn layout(&self) -> &Layout {
        self.placer.layout()
    }

    /// Should text wrap in this [`Ui`]?
    ///
    /// This is determined first by [`Style::wrap`], and then by the layout of this [`Ui`].
    pub fn wrap_text(&self) -> bool {
        if let Some(wrap) = self.style.wrap {
            wrap
        } else if let Some(grid) = self.placer.grid() {
            grid.wrap_text()
        } else {
            let layout = self.layout();
            layout.is_vertical() || layout.is_horizontal() && layout.main_wrap()
        }
    }

    /// Create a painter for a sub-region of this Ui.
    ///
    /// The clip-rect of the returned [`Painter`] will be the intersection
    /// of the given rectangle and the `clip_rect()` of this [`Ui`].
    pub fn painter_at(&self, rect: Rect) -> Painter {
        self.painter().with_clip_rect(rect)
    }

    /// Use this to paint stuff within this [`Ui`].
    #[inline]
    pub fn layer_id(&self) -> LayerId {
        self.painter().layer_id()
    }

    /// The height of text of this text style
    pub fn text_style_height(&self, style: &TextStyle) -> f32 {
        self.fonts(|f| f.row_height(&style.resolve(self.style())))
    }

    /// Screen-space rectangle for clipping what we paint in this ui.
    /// This is used, for instance, to avoid painting outside a window that is smaller than its contents.
    #[inline]
    pub fn clip_rect(&self) -> Rect {
        self.painter.clip_rect()
    }

    /// Screen-space rectangle for clipping what we paint in this ui.
    /// This is used, for instance, to avoid painting outside a window that is smaller than its contents.
    pub fn set_clip_rect(&mut self, clip_rect: Rect) {
        self.painter.set_clip_rect(clip_rect);
    }

    /// Can be used for culling: if `false`, then no part of `rect` will be visible on screen.
    pub fn is_rect_visible(&self, rect: Rect) -> bool {
        self.is_visible() && rect.intersects(self.clip_rect())
    }
}

/// # Helpers for accessing the underlying [`Context`].
/// These functions all lock the [`Context`] owned by this [`Ui`].
/// Please see the documentation of [`Context`] for how locking works!
impl Ui {
    /// Read-only access to the shared [`InputState`].
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// if ui.input(|i| i.key_pressed(egui::Key::A)) {
    ///     // …
    /// }
    /// # });
    /// ```
    #[inline]
    pub fn input<R>(&self, reader: impl FnOnce(&InputState) -> R) -> R {
        self.ctx().input(reader)
    }

    /// Read-write access to the shared [`InputState`].
    #[inline]
    pub fn input_mut<R>(&self, writer: impl FnOnce(&mut InputState) -> R) -> R {
        self.ctx().input_mut(writer)
    }

    /// Read-only access to the shared [`Memory`].
    #[inline]
    pub fn memory<R>(&self, reader: impl FnOnce(&Memory) -> R) -> R {
        self.ctx().memory(reader)
    }

    /// Read-write access to the shared [`Memory`].
    #[inline]
    pub fn memory_mut<R>(&self, writer: impl FnOnce(&mut Memory) -> R) -> R {
        self.ctx().memory_mut(writer)
    }

    /// Read-only access to the shared [`IdTypeMap`], which stores superficial widget state.
    #[inline]
    pub fn data<R>(&self, reader: impl FnOnce(&IdTypeMap) -> R) -> R {
        self.ctx().data(reader)
    }

    /// Read-write access to the shared [`IdTypeMap`], which stores superficial widget state.
    #[inline]
    pub fn data_mut<R>(&self, writer: impl FnOnce(&mut IdTypeMap) -> R) -> R {
        self.ctx().data_mut(writer)
    }

    /// Read-only access to the shared [`PlatformOutput`].
    ///
    /// This is what egui outputs each frame.
    ///
    /// ```
    /// # let mut ctx = egui::Context::default();
    /// ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::Progress);
    /// ```
    #[inline]
    pub fn output<R>(&self, reader: impl FnOnce(&PlatformOutput) -> R) -> R {
        self.ctx().output(reader)
    }

    /// Read-write access to the shared [`PlatformOutput`].
    ///
    /// This is what egui outputs each frame.
    ///
    /// ```
    /// # let mut ctx = egui::Context::default();
    /// ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::Progress);
    /// ```
    #[inline]
    pub fn output_mut<R>(&self, writer: impl FnOnce(&mut PlatformOutput) -> R) -> R {
        self.ctx().output_mut(writer)
    }

    /// Read-only access to [`Fonts`].
    #[inline]
    pub fn fonts<R>(&self, reader: impl FnOnce(&Fonts) -> R) -> R {
        self.ctx().fonts(reader)
    }
}

// ------------------------------------------------------------------------

/// # Sizes etc
impl Ui {
    /// Where and how large the [`Ui`] is already.
    /// All widgets that have been added ot this [`Ui`] fits within this rectangle.
    ///
    /// No matter what, the final Ui will be at least this large.
    ///
    /// This will grow as new widgets are added, but never shrink.
    pub fn min_rect(&self) -> Rect {
        self.placer.min_rect()
    }

    /// Size of content; same as `min_rect().size()`
    pub fn min_size(&self) -> Vec2 {
        self.min_rect().size()
    }

    /// New widgets will *try* to fit within this rectangle.
    ///
    /// Text labels will wrap to fit within `max_rect`.
    /// Separator lines will span the `max_rect`.
    ///
    /// If a new widget doesn't fit within the `max_rect` then the
    /// [`Ui`] will make room for it by expanding both `min_rect` and `max_rect`.
    pub fn max_rect(&self) -> Rect {
        self.placer.max_rect()
    }

    /// Used for animation, kind of hacky
    pub(crate) fn force_set_min_rect(&mut self, min_rect: Rect) {
        self.placer.force_set_min_rect(min_rect);
    }

    // ------------------------------------------------------------------------

    /// Set the maximum size of the ui.
    /// You won't be able to shrink it below the current minimum size.
    pub fn set_max_size(&mut self, size: Vec2) {
        self.set_max_width(size.x);
        self.set_max_height(size.y);
    }

    /// Set the maximum width of the ui.
    /// You won't be able to shrink it below the current minimum size.
    pub fn set_max_width(&mut self, width: f32) {
        self.placer.set_max_width(width);
    }

    /// Set the maximum height of the ui.
    /// You won't be able to shrink it below the current minimum size.
    pub fn set_max_height(&mut self, height: f32) {
        self.placer.set_max_height(height);
    }

    // ------------------------------------------------------------------------

    /// Set the minimum size of the ui.
    /// This can't shrink the ui, only make it larger.
    pub fn set_min_size(&mut self, size: Vec2) {
        self.set_min_width(size.x);
        self.set_min_height(size.y);
    }

    /// Set the minimum width of the ui.
    /// This can't shrink the ui, only make it larger.
    pub fn set_min_width(&mut self, width: f32) {
        self.placer.set_min_width(width);
    }

    /// Set the minimum height of the ui.
    /// This can't shrink the ui, only make it larger.
    pub fn set_min_height(&mut self, height: f32) {
        self.placer.set_min_height(height);
    }

    // ------------------------------------------------------------------------

    /// Helper: shrinks the max width to the current width,
    /// so further widgets will try not to be wider than previous widgets.
    /// Useful for normal vertical layouts.
    pub fn shrink_width_to_current(&mut self) {
        self.set_max_width(self.min_rect().width());
    }

    /// Helper: shrinks the max height to the current height,
    /// so further widgets will try not to be wider than previous widgets.
    pub fn shrink_height_to_current(&mut self) {
        self.set_max_height(self.min_rect().height());
    }

    /// Expand the `min_rect` and `max_rect` of this ui to include a child at the given rect.
    pub fn expand_to_include_rect(&mut self, rect: Rect) {
        self.placer.expand_to_include_rect(rect);
    }

    /// `ui.set_width_range(min..=max);` is equivalent to `ui.set_min_width(min); ui.set_max_width(max);`.
    pub fn set_width_range(&mut self, width: std::ops::RangeInclusive<f32>) {
        self.set_min_width(*width.start());
        self.set_max_width(*width.end());
    }

    /// `ui.set_height_range(min..=max);` is equivalent to `ui.set_min_height(min); ui.set_max_height(max);`.
    pub fn set_height_range(&mut self, height: std::ops::RangeInclusive<f32>) {
        self.set_min_height(*height.start());
        self.set_max_height(*height.end());
    }

    /// Set both the minimum and maximum width.
    pub fn set_width(&mut self, width: f32) {
        self.set_min_width(width);
        self.set_max_width(width);
    }

    /// Set both the minimum and maximum height.
    pub fn set_height(&mut self, height: f32) {
        self.set_min_height(height);
        self.set_max_height(height);
    }

    /// Ensure we are big enough to contain the given x-coordinate.
    /// This is sometimes useful to expand an ui to stretch to a certain place.
    pub fn expand_to_include_x(&mut self, x: f32) {
        self.placer.expand_to_include_x(x);
    }

    /// Ensure we are big enough to contain the given y-coordinate.
    /// This is sometimes useful to expand an ui to stretch to a certain place.
    pub fn expand_to_include_y(&mut self, y: f32) {
        self.placer.expand_to_include_y(y);
    }

    // ------------------------------------------------------------------------
    // Layout related measures:

    /// The available space at the moment, given the current cursor.
    /// This how much more space we can take up without overflowing our parent.
    /// Shrinks as widgets allocate space and the cursor moves.
    /// A small size should be interpreted as "as little as possible".
    /// An infinite size should be interpreted as "as much as you want".
    pub fn available_size(&self) -> Vec2 {
        self.placer.available_size()
    }

    pub fn available_width(&self) -> f32 {
        self.available_size().x
    }

    pub fn available_height(&self) -> f32 {
        self.available_size().y
    }

    /// In case of a wrapping layout, how much space is left on this row/column?
    pub fn available_size_before_wrap(&self) -> Vec2 {
        self.placer.available_rect_before_wrap().size()
    }

    pub fn available_rect_before_wrap(&self) -> Rect {
        self.placer.available_rect_before_wrap()
    }
}

/// # [`Id`] creation
impl Ui {
    /// Use this to generate widget ids for widgets that have persistent state in [`Memory`].
    pub fn make_persistent_id<IdSource>(&self, id_source: IdSource) -> Id
    where
        IdSource: Hash,
    {
        self.id.with(&id_source)
    }

    /// This is the `Id` that will be assigned to the next widget added to this `Ui`.
    pub fn next_auto_id(&self) -> Id {
        Id::new(self.next_auto_id_source)
    }

    pub fn auto_id_with<IdSource>(&self, id_source: IdSource) -> Id
    where
        IdSource: Hash,
    {
        Id::new(self.next_auto_id_source).with(id_source)
    }

    pub fn skip_ahead_auto_ids(&mut self, count: usize) {
        self.next_auto_id_source = self.next_auto_id_source.wrapping_add(count as u64);
    }
}

/// # Interaction
impl Ui {
    /// Check for clicks, drags and/or hover on a specific region of this [`Ui`].
    pub fn interact(&self, rect: Rect, id: Id, sense: Sense) -> Response {
        self.ctx().interact(
            self.clip_rect(),
            self.spacing().item_spacing,
            self.layer_id(),
            id,
            rect,
            sense,
            self.enabled,
        )
    }

    /// Check for clicks, and drags on a specific region that is hovered.
    /// This can be used once you have checked that some shape you are painting has been hovered,
    /// and want to check for clicks and drags on hovered items this frame.
    /// The given [`Rect`] should approximately be where the thing is,
    /// as it is just where warnings will be painted if there is an [`Id`] clash.
    pub fn interact_with_hovered(
        &self,
        rect: Rect,
        hovered: bool,
        id: Id,
        sense: Sense,
    ) -> Response {
        self.ctx()
            .interact_with_hovered(self.layer_id(), id, rect, sense, self.enabled, hovered)
    }

    /// Is the pointer (mouse/touch) above this rectangle in this [`Ui`]?
    ///
    /// The `clip_rect` and layer of this [`Ui`] will be respected, so, for instance,
    /// if this [`Ui`] is behind some other window, this will always return `false`.
    pub fn rect_contains_pointer(&self, rect: Rect) -> bool {
        self.ctx()
            .rect_contains_pointer(self.layer_id(), self.clip_rect().intersect(rect))
    }

    /// Is the pointer (mouse/touch) above this [`Ui`]?
    /// Equivalent to `ui.rect_contains_pointer(ui.min_rect())`
    pub fn ui_contains_pointer(&self) -> bool {
        self.rect_contains_pointer(self.min_rect())
    }
}

/// # Allocating space: where do I put my widgets?
impl Ui {
    /// Allocate space for a widget and check for interaction in the space.
    /// Returns a [`Response`] which contains a rectangle, id, and interaction info.
    ///
    /// ## How sizes are negotiated
    /// Each widget should have a *minimum desired size* and a *desired size*.
    /// When asking for space, ask AT LEAST for your minimum, and don't ask for more than you need.
    /// If you want to fill the space, ask about [`Ui::available_size`] and use that.
    ///
    /// You may get MORE space than you asked for, for instance
    /// for justified layouts, like in menus.
    ///
    /// You will never get a rectangle that is smaller than the amount of space you asked for.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// let response = ui.allocate_response(egui::vec2(100.0, 200.0), egui::Sense::click());
    /// if response.clicked() { /* … */ }
    /// ui.painter().rect_stroke(response.rect, 0.0, (1.0, egui::Color32::WHITE));
    /// # });
    /// ```
    pub fn allocate_response(&mut self, desired_size: Vec2, sense: Sense) -> Response {
        let (id, rect) = self.allocate_space(desired_size);
        self.interact(rect, id, sense)
    }

    /// Returns a [`Rect`] with exactly what you asked for.
    ///
    /// The response rect will be larger if this is part of a justified layout or similar.
    /// This means that if this is a narrow widget in a wide justified layout, then
    /// the widget will react to interactions outside the returned [`Rect`].
    pub fn allocate_exact_size(&mut self, desired_size: Vec2, sense: Sense) -> (Rect, Response) {
        let response = self.allocate_response(desired_size, sense);
        let rect = self
            .placer
            .align_size_within_rect(desired_size, response.rect);
        (rect, response)
    }

    /// Allocate at least as much space as needed, and interact with that rect.
    ///
    /// The returned [`Rect`] will be the same size as `Response::rect`.
    pub fn allocate_at_least(&mut self, desired_size: Vec2, sense: Sense) -> (Rect, Response) {
        let response = self.allocate_response(desired_size, sense);
        (response.rect, response)
    }

    /// Reserve this much space and move the cursor.
    /// Returns where to put the widget.
    ///
    /// ## How sizes are negotiated
    /// Each widget should have a *minimum desired size* and a *desired size*.
    /// When asking for space, ask AT LEAST for your minimum, and don't ask for more than you need.
    /// If you want to fill the space, ask about [`Ui::available_size`] and use that.
    ///
    /// You may get MORE space than you asked for, for instance
    /// for justified layouts, like in menus.
    ///
    /// You will never get a rectangle that is smaller than the amount of space you asked for.
    ///
    /// Returns an automatic [`Id`] (which you can use for interaction) and the [`Rect`] of where to put your widget.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// let (id, rect) = ui.allocate_space(egui::vec2(100.0, 200.0));
    /// let response = ui.interact(rect, id, egui::Sense::click());
    /// # });
    /// ```
    pub fn allocate_space(&mut self, desired_size: Vec2) -> (Id, Rect) {
        // For debug rendering
        let original_available = self.available_size_before_wrap();
        let too_wide = desired_size.x > original_available.x;
        let too_high = desired_size.y > original_available.y;

        let rect = self.allocate_space_impl(desired_size);

        if self.style().debug.debug_on_hover && self.rect_contains_pointer(rect) {
            let painter = self.ctx().debug_painter();
            painter.rect_stroke(rect, 4.0, (1.0, Color32::LIGHT_BLUE));
            self.placer.debug_paint_cursor(&painter, "next");
        }

        let debug_expand_width = self.style().debug.show_expand_width;
        let debug_expand_height = self.style().debug.show_expand_height;

        if (debug_expand_width && too_wide) || (debug_expand_height && too_high) {
            self.painter
                .rect_stroke(rect, 0.0, (1.0, Color32::LIGHT_BLUE));

            let stroke = Stroke::new(2.5, Color32::from_rgb(200, 0, 0));
            let paint_line_seg = |a, b| self.painter().line_segment([a, b], stroke);

            if debug_expand_width && too_wide {
                paint_line_seg(rect.left_top(), rect.left_bottom());
                paint_line_seg(rect.left_center(), rect.right_center());
                paint_line_seg(
                    pos2(rect.left() + original_available.x, rect.top()),
                    pos2(rect.left() + original_available.x, rect.bottom()),
                );
                paint_line_seg(rect.right_top(), rect.right_bottom());
            }

            if debug_expand_height && too_high {
                paint_line_seg(rect.left_top(), rect.right_top());
                paint_line_seg(rect.center_top(), rect.center_bottom());
                paint_line_seg(rect.left_bottom(), rect.right_bottom());
            }
        }

        let id = Id::new(self.next_auto_id_source);
        self.next_auto_id_source = self.next_auto_id_source.wrapping_add(1);

        (id, rect)
    }

    /// Reserve this much space and move the cursor.
    /// Returns where to put the widget.
    fn allocate_space_impl(&mut self, desired_size: Vec2) -> Rect {
        let item_spacing = self.spacing().item_spacing;
        let frame_rect = self.placer.next_space(desired_size, item_spacing);
        egui_assert!(!frame_rect.any_nan());
        let widget_rect = self.placer.justify_and_align(frame_rect, desired_size);

        self.placer
            .advance_after_rects(frame_rect, widget_rect, item_spacing);

        widget_rect
    }

    /// Allocate a specific part of the [`Ui`].
    ///
    /// Ignore the layout of the [`Ui`]: just put my widget here!
    /// The layout cursor will advance to past this `rect`.
    pub fn allocate_rect(&mut self, rect: Rect, sense: Sense) -> Response {
        let id = self.advance_cursor_after_rect(rect);
        self.interact(rect, id, sense)
    }

    pub(crate) fn advance_cursor_after_rect(&mut self, rect: Rect) -> Id {
        egui_assert!(!rect.any_nan());
        let item_spacing = self.spacing().item_spacing;
        self.placer.advance_after_rects(rect, rect, item_spacing);

        if self.style().debug.debug_on_hover && self.rect_contains_pointer(rect) {
            let painter = self.ctx().debug_painter();
            painter.rect_stroke(rect, 4.0, (1.0, Color32::LIGHT_BLUE));
            self.placer.debug_paint_cursor(&painter, "next");
        }

        let id = Id::new(self.next_auto_id_source);
        self.next_auto_id_source = self.next_auto_id_source.wrapping_add(1);
        id
    }

    pub(crate) fn placer(&self) -> &Placer {
        &self.placer
    }

    /// Where the next widget will be put.
    ///
    /// One side of this will always be infinite: the direction in which new widgets will be added.
    /// The opposing side is what is incremented.
    /// The crossing sides are initialized to `max_rect`.
    ///
    /// So one can think of `cursor` as a constraint on the available region.
    ///
    /// If something has already been added, this will point to `style.spacing.item_spacing` beyond the latest child.
    /// The cursor can thus be `style.spacing.item_spacing` pixels outside of the `min_rect`.
    pub fn cursor(&self) -> Rect {
        self.placer.cursor()
    }

    pub(crate) fn set_cursor(&mut self, cursor: Rect) {
        self.placer.set_cursor(cursor);
    }

    /// Where do we expect a zero-sized widget to be placed?
    pub fn next_widget_position(&self) -> Pos2 {
        self.placer.next_widget_position()
    }

    /// Allocated the given space and then adds content to that space.
    /// If the contents overflow, more space will be allocated.
    /// When finished, the amount of space actually used (`min_rect`) will be allocated.
    /// So you can request a lot of space and then use less.
    #[inline]
    pub fn allocate_ui<R>(
        &mut self,
        desired_size: Vec2,
        add_contents: impl FnOnce(&mut Self) -> R,
    ) -> InnerResponse<R> {
        self.allocate_ui_with_layout(desired_size, *self.layout(), add_contents)
    }

    /// Allocated the given space and then adds content to that space.
    /// If the contents overflow, more space will be allocated.
    /// When finished, the amount of space actually used (`min_rect`) will be allocated.
    /// So you can request a lot of space and then use less.
    #[inline]
    pub fn allocate_ui_with_layout<R>(
        &mut self,
        desired_size: Vec2,
        layout: Layout,
        add_contents: impl FnOnce(&mut Self) -> R,
    ) -> InnerResponse<R> {
        self.allocate_ui_with_layout_dyn(desired_size, layout, Box::new(add_contents))
    }

    fn allocate_ui_with_layout_dyn<'c, R>(
        &mut self,
        desired_size: Vec2,
        layout: Layout,
        add_contents: Box<dyn FnOnce(&mut Self) -> R + 'c>,
    ) -> InnerResponse<R> {
        crate::egui_assert!(desired_size.x >= 0.0 && desired_size.y >= 0.0);
        let item_spacing = self.spacing().item_spacing;
        let frame_rect = self.placer.next_space(desired_size, item_spacing);
        let child_rect = self.placer.justify_and_align(frame_rect, desired_size);

        let mut child_ui = self.child_ui(child_rect, layout);
        let ret = add_contents(&mut child_ui);
        let final_child_rect = child_ui.min_rect();

        self.placer
            .advance_after_rects(final_child_rect, final_child_rect, item_spacing);

        if self.style().debug.debug_on_hover && self.rect_contains_pointer(final_child_rect) {
            let painter = self.ctx().debug_painter();
            painter.rect_stroke(frame_rect, 4.0, (1.0, Color32::LIGHT_BLUE));
            painter.rect_stroke(final_child_rect, 4.0, (1.0, Color32::LIGHT_BLUE));
            self.placer.debug_paint_cursor(&painter, "next");
        }

        let response = self.interact(final_child_rect, child_ui.id, Sense::hover());
        InnerResponse::new(ret, response)
    }

    /// Allocated the given rectangle and then adds content to that rectangle.
    /// If the contents overflow, more space will be allocated.
    /// When finished, the amount of space actually used (`min_rect`) will be allocated.
    /// So you can request a lot of space and then use less.
    pub fn allocate_ui_at_rect<R>(
        &mut self,
        max_rect: Rect,
        add_contents: impl FnOnce(&mut Self) -> R,
    ) -> InnerResponse<R> {
        egui_assert!(max_rect.is_finite());
        let mut child_ui = self.child_ui(max_rect, *self.layout());
        let ret = add_contents(&mut child_ui);
        let final_child_rect = child_ui.min_rect();

        self.placer.advance_after_rects(
            final_child_rect,
            final_child_rect,
            self.spacing().item_spacing,
        );

        let response = self.interact(final_child_rect, child_ui.id, Sense::hover());
        InnerResponse::new(ret, response)
    }

    /// Convenience function to get a region to paint on.
    ///
    /// Note that egui uses screen coordinates for everything.
    ///
    /// ```
    /// # use egui::*;
    /// # use std::f32::consts::TAU;
    /// # egui::__run_test_ui(|ui| {
    /// let size = Vec2::splat(16.0);
    /// let (response, painter) = ui.allocate_painter(size, Sense::hover());
    /// let rect = response.rect;
    /// let c = rect.center();
    /// let r = rect.width() / 2.0 - 1.0;
    /// let color = Color32::from_gray(128);
    /// let stroke = Stroke::new(1.0, color);
    /// painter.circle_stroke(c, r, stroke);
    /// painter.line_segment([c - vec2(0.0, r), c + vec2(0.0, r)], stroke);
    /// painter.line_segment([c, c + r * Vec2::angled(TAU * 1.0 / 8.0)], stroke);
    /// painter.line_segment([c, c + r * Vec2::angled(TAU * 3.0 / 8.0)], stroke);
    /// # });
    /// ```
    pub fn allocate_painter(&mut self, desired_size: Vec2, sense: Sense) -> (Response, Painter) {
        let response = self.allocate_response(desired_size, sense);
        let clip_rect = self.clip_rect().intersect(response.rect); // Make sure we don't paint out of bounds
        let painter = Painter::new(self.ctx().clone(), self.layer_id(), clip_rect);
        (response, painter)
    }

    /// Adjust the scroll position of any parent [`ScrollArea`] so that the given [`Rect`] becomes visible.
    ///
    /// If `align` is `None`, it'll scroll enough to bring the cursor into view.
    ///
    /// See also: [`Response::scroll_to_me`], [`Ui::scroll_to_cursor`]. [`Ui::scroll_with_delta`]..
    ///
    /// ```
    /// # use egui::Align;
    /// # egui::__run_test_ui(|ui| {
    /// egui::ScrollArea::vertical().show(ui, |ui| {
    ///     // …
    ///     let response = ui.button("Center on me.");
    ///     if response.clicked() {
    ///         ui.scroll_to_rect(response.rect, Some(Align::Center));
    ///     }
    /// });
    /// # });
    /// ```
    pub fn scroll_to_rect(&self, rect: Rect, align: Option<Align>) {
        for d in 0..2 {
            let range = rect.min[d]..=rect.max[d];
            self.ctx()
                .frame_state_mut(|state| state.scroll_target[d] = Some((range, align)));
        }
    }

    /// Adjust the scroll position of any parent [`ScrollArea`] so that the cursor (where the next widget goes) becomes visible.
    ///
    /// If `align` is not provided, it'll scroll enough to bring the cursor into view.
    ///
    /// See also: [`Response::scroll_to_me`], [`Ui::scroll_to_rect`]. [`Ui::scroll_with_delta`].
    ///
    /// ```
    /// # use egui::Align;
    /// # egui::__run_test_ui(|ui| {
    /// egui::ScrollArea::vertical().show(ui, |ui| {
    ///     let scroll_bottom = ui.button("Scroll to bottom.").clicked();
    ///     for i in 0..1000 {
    ///         ui.label(format!("Item {}", i));
    ///     }
    ///
    ///     if scroll_bottom {
    ///         ui.scroll_to_cursor(Some(Align::BOTTOM));
    ///     }
    /// });
    /// # });
    /// ```
    pub fn scroll_to_cursor(&self, align: Option<Align>) {
        let target = self.next_widget_position();
        for d in 0..2 {
            let target = target[d];
            self.ctx()
                .frame_state_mut(|state| state.scroll_target[d] = Some((target..=target, align)));
        }
    }

    /// Scroll this many points in the given direction, in the parent [`ScrollArea`].
    ///
    /// The delta dictates how the _content_ (i.e. this UI) should move.
    ///
    /// A positive X-value indicates the content is being moved right,
    /// as when swiping right on a touch-screen or track-pad with natural scrolling.
    ///
    /// A positive Y-value indicates the content is being moved down,
    /// as when swiping down on a touch-screen or track-pad with natural scrolling.
    ///
    /// /// See also: [`Response::scroll_to_me`], [`Ui::scroll_to_rect`], [`Ui::scroll_to_cursor`]
    ///
    /// ```
    /// # use egui::{Align, Vec2};
    /// # egui::__run_test_ui(|ui| {
    /// let mut scroll_delta = Vec2::ZERO;
    /// if ui.button("Scroll down").clicked() {
    ///     scroll_delta.y -= 64.0; // move content up
    /// }
    /// egui::ScrollArea::vertical().show(ui, |ui| {
    ///     ui.scroll_with_delta(scroll_delta);
    ///     for i in 0..1000 {
    ///         ui.label(format!("Item {}", i));
    ///     }
    /// });
    /// # });
    /// ```
    pub fn scroll_with_delta(&self, delta: Vec2) {
        self.ctx()
            .frame_state_mut(|state| state.scroll_delta += delta);
    }
}

/// # Adding widgets
impl Ui {
    /// Add a [`Widget`] to this [`Ui`] at a location dependent on the current [`Layout`].
    ///
    /// The returned [`Response`] can be used to check for interactions,
    /// as well as adding tooltips using [`Response::on_hover_text`].
    ///
    /// See also [`Self::add_sized`] and [`Self::put`].
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut my_value = 42;
    /// let response = ui.add(egui::Slider::new(&mut my_value, 0..=100));
    /// response.on_hover_text("Drag me!");
    /// # });
    /// ```
    #[inline]
    pub fn add(&mut self, widget: impl Widget) -> Response {
        widget.ui(self)
    }

    /// Add a [`Widget`] to this [`Ui`] with a given size.
    /// The widget will attempt to fit within the given size, but some widgets may overflow.
    ///
    /// To fill all remaining area, use `ui.add_sized(ui.available_size(), widget);`
    ///
    /// See also [`Self::add`] and [`Self::put`].
    ///
    /// ```
    /// # let mut my_value = 42;
    /// # egui::__run_test_ui(|ui| {
    /// ui.add_sized([40.0, 20.0], egui::DragValue::new(&mut my_value));
    /// # });
    /// ```
    pub fn add_sized(&mut self, max_size: impl Into<Vec2>, widget: impl Widget) -> Response {
        // TODO(emilk): configure to overflow to main_dir instead of centered overflow
        // to handle the bug mentioned at https://github.com/emilk/egui/discussions/318#discussioncomment-627578
        // and fixed in https://github.com/emilk/egui/commit/035166276322b3f2324bd8b97ffcedc63fa8419f
        //
        // Make sure we keep the same main direction since it changes e.g. how text is wrapped:
        let layout = Layout::centered_and_justified(self.layout().main_dir());
        self.allocate_ui_with_layout(max_size.into(), layout, |ui| ui.add(widget))
            .inner
    }

    /// Add a [`Widget`] to this [`Ui`] at a specific location (manual layout).
    ///
    /// See also [`Self::add`] and [`Self::add_sized`].
    pub fn put(&mut self, max_rect: Rect, widget: impl Widget) -> Response {
        self.allocate_ui_at_rect(max_rect, |ui| {
            ui.centered_and_justified(|ui| ui.add(widget)).inner
        })
        .inner
    }

    /// Add a single [`Widget`] that is possibly disabled, i.e. greyed out and non-interactive.
    ///
    /// If you call `add_enabled` from within an already disabled [`Ui`],
    /// the widget will always be disabled, even if the `enabled` argument is true.
    ///
    /// See also [`Self::add_enabled_ui`] and [`Self::is_enabled`].
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// ui.add_enabled(false, egui::Button::new("Can't click this"));
    /// # });
    /// ```
    pub fn add_enabled(&mut self, enabled: bool, widget: impl Widget) -> Response {
        if self.is_enabled() && !enabled {
            let old_painter = self.painter.clone();
            self.set_enabled(false);
            let response = self.add(widget);
            self.enabled = true;
            self.painter = old_painter;
            response
        } else {
            self.add(widget)
        }
    }

    /// Add a section that is possibly disabled, i.e. greyed out and non-interactive.
    ///
    /// If you call `add_enabled_ui` from within an already disabled [`Ui`],
    /// the result will always be disabled, even if the `enabled` argument is true.
    ///
    /// See also [`Self::add_enabled`] and [`Self::is_enabled`].
    ///
    /// ### Example
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut enabled = true;
    /// ui.checkbox(&mut enabled, "Enable subsection");
    /// ui.add_enabled_ui(enabled, |ui| {
    ///     if ui.button("Button that is not always clickable").clicked() {
    ///         /* … */
    ///     }
    /// });
    /// # });
    /// ```
    pub fn add_enabled_ui<R>(
        &mut self,
        enabled: bool,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.scope(|ui| {
            ui.set_enabled(enabled);
            add_contents(ui)
        })
    }

    /// Add a single [`Widget`] that is possibly invisible.
    ///
    /// An invisible widget still takes up the same space as if it were visible.
    ///
    /// If you call `add_visible` from within an already invisible [`Ui`],
    /// the widget will always be invisible, even if the `visible` argument is true.
    ///
    /// See also [`Self::add_visible_ui`], [`Self::set_visible`] and [`Self::is_visible`].
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// ui.add_visible(false, egui::Label::new("You won't see me!"));
    /// # });
    /// ```
    pub fn add_visible(&mut self, visible: bool, widget: impl Widget) -> Response {
        if self.is_visible() && !visible {
            // temporary make us invisible:
            let old_painter = self.painter.clone();
            let old_enabled = self.enabled;

            self.set_visible(false);

            let response = self.add(widget);

            self.painter = old_painter;
            self.enabled = old_enabled;
            response
        } else {
            self.add(widget)
        }
    }

    /// Add a section that is possibly invisible, i.e. greyed out and non-interactive.
    ///
    /// An invisible ui still takes up the same space as if it were visible.
    ///
    /// If you call `add_visible_ui` from within an already invisible [`Ui`],
    /// the result will always be invisible, even if the `visible` argument is true.
    ///
    /// See also [`Self::add_visible`], [`Self::set_visible`] and [`Self::is_visible`].
    ///
    /// ### Example
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut visible = true;
    /// ui.checkbox(&mut visible, "Show subsection");
    /// ui.add_visible_ui(visible, |ui| {
    ///     ui.label("Maybe you see this, maybe you don't!");
    /// });
    /// # });
    /// ```
    pub fn add_visible_ui<R>(
        &mut self,
        visible: bool,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.scope(|ui| {
            ui.set_visible(visible);
            add_contents(ui)
        })
    }

    /// Add extra space before the next widget.
    ///
    /// The direction is dependent on the layout.
    /// This will be in addition to the [`crate::style::Spacing::item_spacing`].
    ///
    /// [`Self::min_rect`] will expand to contain the space.
    #[inline]
    pub fn add_space(&mut self, amount: f32) {
        self.placer.advance_cursor(amount);
    }

    /// Show some text.
    ///
    /// Shortcut for `add(Label::new(text))`
    ///
    /// See also [`Label`].
    ///
    /// ### Example
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// use egui::{RichText, FontId, Color32};
    /// ui.label("Normal text");
    /// ui.label(RichText::new("Large text").font(FontId::proportional(40.0)));
    /// ui.label(RichText::new("Red text").color(Color32::RED));
    /// # });
    /// ```
    #[inline]
    pub fn label(&mut self, text: impl Into<WidgetText>) -> Response {
        Label::new(text).ui(self)
    }

    /// Show colored text.
    ///
    /// Shortcut for `ui.label(RichText::new(text).color(color))`
    pub fn colored_label(
        &mut self,
        color: impl Into<Color32>,
        text: impl Into<RichText>,
    ) -> Response {
        Label::new(text.into().color(color)).ui(self)
    }

    /// Show large text.
    ///
    /// Shortcut for `ui.label(RichText::new(text).heading())`
    pub fn heading(&mut self, text: impl Into<RichText>) -> Response {
        Label::new(text.into().heading()).ui(self)
    }

    /// Show monospace (fixed width) text.
    ///
    /// Shortcut for `ui.label(RichText::new(text).monospace())`
    pub fn monospace(&mut self, text: impl Into<RichText>) -> Response {
        Label::new(text.into().monospace()).ui(self)
    }

    /// Show text as monospace with a gray background.
    ///
    /// Shortcut for `ui.label(RichText::new(text).code())`
    pub fn code(&mut self, text: impl Into<RichText>) -> Response {
        Label::new(text.into().code()).ui(self)
    }

    /// Show small text.
    ///
    /// Shortcut for `ui.label(RichText::new(text).small())`
    pub fn small(&mut self, text: impl Into<RichText>) -> Response {
        Label::new(text.into().small()).ui(self)
    }

    /// Show text that stand out a bit (e.g. slightly brighter).
    ///
    /// Shortcut for `ui.label(RichText::new(text).strong())`
    pub fn strong(&mut self, text: impl Into<RichText>) -> Response {
        Label::new(text.into().strong()).ui(self)
    }

    /// Show text that is weaker (fainter color).
    ///
    /// Shortcut for `ui.label(RichText::new(text).weak())`
    pub fn weak(&mut self, text: impl Into<RichText>) -> Response {
        Label::new(text.into().weak()).ui(self)
    }

    /// Looks like a hyperlink.
    ///
    /// Shortcut for `add(Link::new(text))`.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// if ui.link("Documentation").clicked() {
    ///     // …
    /// }
    /// # });
    /// ```
    ///
    /// See also [`Link`].
    #[must_use = "You should check if the user clicked this with `if ui.link(…).clicked() { … } "]
    pub fn link(&mut self, text: impl Into<WidgetText>) -> Response {
        Link::new(text).ui(self)
    }

    /// Link to a web page.
    ///
    /// Shortcut for `add(Hyperlink::new(url))`.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// ui.hyperlink("https://www.egui.rs/");
    /// # });
    /// ```
    ///
    /// See also [`Hyperlink`].
    pub fn hyperlink(&mut self, url: impl ToString) -> Response {
        Hyperlink::new(url).ui(self)
    }

    /// Shortcut for `add(Hyperlink::new(url).text(label))`.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// ui.hyperlink_to("egui on GitHub", "https://www.github.com/emilk/egui/");
    /// # });
    /// ```
    ///
    /// See also [`Hyperlink`].
    pub fn hyperlink_to(&mut self, label: impl Into<WidgetText>, url: impl ToString) -> Response {
        Hyperlink::from_label_and_url(label, url).ui(self)
    }

    /// No newlines (`\n`) allowed. Pressing enter key will result in the [`TextEdit`] losing focus (`response.lost_focus`).
    ///
    /// See also [`TextEdit`].
    pub fn text_edit_singleline<S: widgets::text_edit::TextBuffer>(
        &mut self,
        text: &mut S,
    ) -> Response {
        TextEdit::singleline(text).ui(self)
    }

    /// A [`TextEdit`] for multiple lines. Pressing enter key will create a new line.
    ///
    /// See also [`TextEdit`].
    pub fn text_edit_multiline<S: widgets::text_edit::TextBuffer>(
        &mut self,
        text: &mut S,
    ) -> Response {
        TextEdit::multiline(text).ui(self)
    }

    /// A [`TextEdit`] for code editing.
    ///
    /// This will be multiline, monospace, and will insert tabs instead of moving focus.
    ///
    /// See also [`TextEdit::code_editor`].
    pub fn code_editor<S: widgets::text_edit::TextBuffer>(&mut self, text: &mut S) -> Response {
        self.add(TextEdit::multiline(text).code_editor())
    }

    /// Usage: `if ui.button("Click me").clicked() { … }`
    ///
    /// Shortcut for `add(Button::new(text))`
    ///
    /// See also [`Button`].
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// if ui.button("Click me!").clicked() {
    ///     // …
    /// }
    ///
    /// # use egui::{RichText, Color32};
    /// if ui.button(RichText::new("delete").color(Color32::RED)).clicked() {
    ///     // …
    /// }
    /// # });
    /// ```
    #[must_use = "You should check if the user clicked this with `if ui.button(…).clicked() { … } "]
    #[inline]
    pub fn button(&mut self, text: impl Into<WidgetText>) -> Response {
        Button::new(text).ui(self)
    }

    /// A button as small as normal body text.
    ///
    /// Usage: `if ui.small_button("Click me").clicked() { … }`
    ///
    /// Shortcut for `add(Button::new(text).small())`
    #[must_use = "You should check if the user clicked this with `if ui.small_button(…).clicked() { … } "]
    pub fn small_button(&mut self, text: impl Into<WidgetText>) -> Response {
        Button::new(text).small().ui(self)
    }

    /// Show a checkbox.
    ///
    /// See also [`Self::toggle_value`].
    #[inline]
    pub fn checkbox(&mut self, checked: &mut bool, text: impl Into<WidgetText>) -> Response {
        Checkbox::new(checked, text).ui(self)
    }

    /// Acts like a checkbox, but looks like a [`SelectableLabel`].
    ///
    /// Click to toggle to bool.
    ///
    /// See also [`Self::checkbox`].
    pub fn toggle_value(&mut self, selected: &mut bool, text: impl Into<WidgetText>) -> Response {
        let mut response = self.selectable_label(*selected, text);
        if response.clicked() {
            *selected = !*selected;
            response.mark_changed();
        }
        response
    }

    /// Show a [`RadioButton`].
    /// Often you want to use [`Self::radio_value`] instead.
    #[must_use = "You should check if the user clicked this with `if ui.radio(…).clicked() { … } "]
    #[inline]
    pub fn radio(&mut self, selected: bool, text: impl Into<WidgetText>) -> Response {
        RadioButton::new(selected, text).ui(self)
    }

    /// Show a [`RadioButton`]. It is selected if `*current_value == selected_value`.
    /// If clicked, `selected_value` is assigned to `*current_value`.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    ///
    /// #[derive(PartialEq)]
    /// enum Enum { First, Second, Third }
    /// let mut my_enum = Enum::First;
    ///
    /// ui.radio_value(&mut my_enum, Enum::First, "First");
    ///
    /// // is equivalent to:
    ///
    /// if ui.add(egui::RadioButton::new(my_enum == Enum::First, "First")).clicked() {
    ///     my_enum = Enum::First
    /// }
    /// # });
    /// ```
    pub fn radio_value<Value: PartialEq>(
        &mut self,
        current_value: &mut Value,
        alternative: Value,
        text: impl Into<WidgetText>,
    ) -> Response {
        let mut response = self.radio(*current_value == alternative, text);
        if response.clicked() && *current_value != alternative {
            *current_value = alternative;
            response.mark_changed();
        }
        response
    }

    /// Show a label which can be selected or not.
    ///
    /// See also [`SelectableLabel`] and [`Self::toggle_value`].
    #[must_use = "You should check if the user clicked this with `if ui.selectable_label(…).clicked() { … } "]
    pub fn selectable_label(&mut self, checked: bool, text: impl Into<WidgetText>) -> Response {
        SelectableLabel::new(checked, text).ui(self)
    }

    /// Show selectable text. It is selected if `*current_value == selected_value`.
    /// If clicked, `selected_value` is assigned to `*current_value`.
    ///
    /// Example: `ui.selectable_value(&mut my_enum, Enum::Alternative, "Alternative")`.
    ///
    /// See also [`SelectableLabel`] and [`Self::toggle_value`].
    pub fn selectable_value<Value: PartialEq>(
        &mut self,
        current_value: &mut Value,
        selected_value: Value,
        text: impl Into<WidgetText>,
    ) -> Response {
        let mut response = self.selectable_label(*current_value == selected_value, text);
        if response.clicked() && *current_value != selected_value {
            *current_value = selected_value;
            response.mark_changed();
        }
        response
    }

    /// Shortcut for `add(Separator::default())`
    ///
    /// See also [`Separator`].
    #[inline]
    pub fn separator(&mut self) -> Response {
        Separator::default().ui(self)
    }

    /// Shortcut for `add(Spinner::new())`
    ///
    /// See also [`Spinner`].
    #[inline]
    pub fn spinner(&mut self) -> Response {
        Spinner::new().ui(self)
    }

    /// Modify an angle. The given angle should be in radians, but is shown to the user in degrees.
    /// The angle is NOT wrapped, so the user may select, for instance 720° = 2𝞃 = 4π
    pub fn drag_angle(&mut self, radians: &mut f32) -> Response {
        let mut degrees = radians.to_degrees();
        let mut response = self.add(DragValue::new(&mut degrees).speed(1.0).suffix("°"));

        // only touch `*radians` if we actually changed the degree value
        if degrees != radians.to_degrees() {
            *radians = degrees.to_radians();
            response.changed = true;
        }

        response
    }

    /// Modify an angle. The given angle should be in radians,
    /// but is shown to the user in fractions of one Tau (i.e. fractions of one turn).
    /// The angle is NOT wrapped, so the user may select, for instance 2𝞃 (720°)
    pub fn drag_angle_tau(&mut self, radians: &mut f32) -> Response {
        use std::f32::consts::TAU;

        let mut taus = *radians / TAU;
        let mut response = self.add(DragValue::new(&mut taus).speed(0.01).suffix("τ"));

        if self.style().explanation_tooltips {
            response =
                response.on_hover_text("1τ = one turn, 0.5τ = half a turn, etc. 0.25τ = 90°");
        }

        // only touch `*radians` if we actually changed the value
        if taus != *radians / TAU {
            *radians = taus * TAU;
            response.changed = true;
        }

        response
    }

    /// Show an image here with the given size.
    ///
    /// In order to display an image you must first acquire a [`TextureHandle`].
    /// This is best done with [`egui_extras::RetainedImage`](https://docs.rs/egui_extras/latest/egui_extras/image/struct.RetainedImage.html) or [`Context::load_texture`].
    ///
    /// ```
    /// struct MyImage {
    ///     texture: Option<egui::TextureHandle>,
    /// }
    ///
    /// impl MyImage {
    ///     fn ui(&mut self, ui: &mut egui::Ui) {
    ///         let texture: &egui::TextureHandle = self.texture.get_or_insert_with(|| {
    ///             // Load the texture only once.
    ///             ui.ctx().load_texture(
    ///                 "my-image",
    ///                 egui::ColorImage::example(),
    ///                 Default::default()
    ///             )
    ///         });
    ///
    ///         // Show the image:
    ///         ui.image(texture, texture.size_vec2());
    ///     }
    /// }
    /// ```
    ///
    /// See also [`crate::Image`] and [`crate::ImageButton`].
    #[inline]
    pub fn image(&mut self, texture_id: impl Into<TextureId>, size: impl Into<Vec2>) -> Response {
        Image::new(texture_id, size).ui(self)
    }
}

/// # Colors
impl Ui {
    /// Shows a button with the given color.
    /// If the user clicks the button, a full color picker is shown.
    pub fn color_edit_button_srgba(&mut self, srgba: &mut Color32) -> Response {
        color_picker::color_edit_button_srgba(self, srgba, color_picker::Alpha::BlendOrAdditive)
    }

    /// Shows a button with the given color.
    /// If the user clicks the button, a full color picker is shown.
    pub fn color_edit_button_hsva(&mut self, hsva: &mut Hsva) -> Response {
        color_picker::color_edit_button_hsva(self, hsva, color_picker::Alpha::BlendOrAdditive)
    }

    /// Shows a button with the given color.
    /// If the user clicks the button, a full color picker is shown.
    /// The given color is in `sRGB` space.
    pub fn color_edit_button_srgb(&mut self, srgb: &mut [u8; 3]) -> Response {
        color_picker::color_edit_button_srgb(self, srgb)
    }

    /// Shows a button with the given color.
    /// If the user clicks the button, a full color picker is shown.
    /// The given color is in linear RGB space.
    pub fn color_edit_button_rgb(&mut self, rgb: &mut [f32; 3]) -> Response {
        color_picker::color_edit_button_rgb(self, rgb)
    }

    /// Shows a button with the given color.
    /// If the user clicks the button, a full color picker is shown.
    /// The given color is in `sRGBA` space with premultiplied alpha
    pub fn color_edit_button_srgba_premultiplied(&mut self, srgba: &mut [u8; 4]) -> Response {
        let mut color = Color32::from_rgba_premultiplied(srgba[0], srgba[1], srgba[2], srgba[3]);
        let response = self.color_edit_button_srgba(&mut color);
        *srgba = color.to_array();
        response
    }

    /// Shows a button with the given color.
    /// If the user clicks the button, a full color picker is shown.
    /// The given color is in `sRGBA` space without premultiplied alpha.
    /// If unsure, what "premultiplied alpha" is, then this is probably the function you want to use.
    pub fn color_edit_button_srgba_unmultiplied(&mut self, srgba: &mut [u8; 4]) -> Response {
        let mut rgba = Rgba::from_srgba_unmultiplied(srgba[0], srgba[1], srgba[2], srgba[3]);
        let response =
            color_picker::color_edit_button_rgba(self, &mut rgba, color_picker::Alpha::OnlyBlend);
        *srgba = rgba.to_srgba_unmultiplied();
        response
    }

    /// Shows a button with the given color.
    /// If the user clicks the button, a full color picker is shown.
    /// The given color is in linear RGBA space with premultiplied alpha
    pub fn color_edit_button_rgba_premultiplied(&mut self, rgba_premul: &mut [f32; 4]) -> Response {
        let mut rgba = Rgba::from_rgba_premultiplied(
            rgba_premul[0],
            rgba_premul[1],
            rgba_premul[2],
            rgba_premul[3],
        );
        let response = color_picker::color_edit_button_rgba(
            self,
            &mut rgba,
            color_picker::Alpha::BlendOrAdditive,
        );
        *rgba_premul = rgba.to_array();
        response
    }

    /// Shows a button with the given color.
    /// If the user clicks the button, a full color picker is shown.
    /// The given color is in linear RGBA space without premultiplied alpha.
    /// If unsure, what "premultiplied alpha" is, then this is probably the function you want to use.
    pub fn color_edit_button_rgba_unmultiplied(&mut self, rgba_unmul: &mut [f32; 4]) -> Response {
        let mut rgba = Rgba::from_rgba_unmultiplied(
            rgba_unmul[0],
            rgba_unmul[1],
            rgba_unmul[2],
            rgba_unmul[3],
        );
        let response =
            color_picker::color_edit_button_rgba(self, &mut rgba, color_picker::Alpha::OnlyBlend);
        *rgba_unmul = rgba.to_rgba_unmultiplied();
        response
    }
}

/// # Adding Containers / Sub-uis:
impl Ui {
    /// Put into a [`Frame::group`], visually grouping the contents together
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// ui.group(|ui| {
    ///     ui.label("Within a frame");
    /// });
    /// # });
    /// ```
    ///
    /// Se also [`Self::scope`].
    pub fn group<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        crate::Frame::group(self.style()).show(self, add_contents)
    }

    /// Create a child Ui with an explicit [`Id`].
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// for i in 0..10 {
    ///     // ui.collapsing("Same header", |ui| { }); // this will cause an ID clash because of the same title!
    ///
    ///     ui.push_id(i, |ui| {
    ///         ui.collapsing("Same header", |ui| { }); // this is fine!
    ///     });
    /// }
    /// # });
    /// ```
    pub fn push_id<R>(
        &mut self,
        id_source: impl Hash,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.scope_dyn(Box::new(add_contents), Id::new(id_source))
    }

    /// Create a scoped child ui.
    ///
    /// You can use this to temporarily change the [`Style`] of a sub-region, for instance:
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// ui.scope(|ui| {
    ///     ui.spacing_mut().slider_width = 200.0; // Temporary change
    ///     // …
    /// });
    /// # });
    /// ```
    pub fn scope<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        self.scope_dyn(Box::new(add_contents), Id::new("child"))
    }

    fn scope_dyn<'c, R>(
        &mut self,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
        id_source: Id,
    ) -> InnerResponse<R> {
        let child_rect = self.available_rect_before_wrap();
        let next_auto_id_source = self.next_auto_id_source;
        let mut child_ui = self.child_ui_with_id_source(child_rect, *self.layout(), id_source);
        self.next_auto_id_source = next_auto_id_source; // HACK: we want `scope` to only increment this once, so that `ui.scope` is equivalent to `ui.allocate_space`.
        let ret = add_contents(&mut child_ui);
        let response = self.allocate_rect(child_ui.min_rect(), Sense::hover());
        InnerResponse::new(ret, response)
    }

    /// Redirect shapes to another paint layer.
    pub fn with_layer_id<R>(
        &mut self,
        layer_id: LayerId,
        add_contents: impl FnOnce(&mut Self) -> R,
    ) -> InnerResponse<R> {
        self.scope(|ui| {
            ui.painter.set_layer_id(layer_id);
            add_contents(ui)
        })
    }

    /// A [`CollapsingHeader`] that starts out collapsed.
    pub fn collapsing<R>(
        &mut self,
        heading: impl Into<WidgetText>,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> CollapsingResponse<R> {
        CollapsingHeader::new(heading).show(self, add_contents)
    }

    /// Create a child ui which is indented to the right.
    ///
    /// The `id_source` here be anything at all.
    // TODO(emilk): remove `id_source` argument?
    #[inline]
    pub fn indent<R>(
        &mut self,
        id_source: impl Hash,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.indent_dyn(id_source, Box::new(add_contents))
    }

    fn indent_dyn<'c, R>(
        &mut self,
        id_source: impl Hash,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<R> {
        assert!(
            self.layout().is_vertical(),
            "You can only indent vertical layouts, found {:?}",
            self.layout()
        );

        let indent = self.spacing().indent;
        let mut child_rect = self.placer.available_rect_before_wrap();
        child_rect.min.x += indent;

        let mut child_ui = Self {
            id: self.id.with(id_source),
            ..self.child_ui(child_rect, *self.layout())
        };
        let ret = add_contents(&mut child_ui);

        let left_vline = self.visuals().indent_has_left_vline;
        let end_with_horizontal_line = self.spacing().indent_ends_with_horizontal_line;

        if left_vline || end_with_horizontal_line {
            if end_with_horizontal_line {
                child_ui.add_space(4.0);
            }

            let stroke = self.visuals().widgets.noninteractive.bg_stroke;
            let left_top = child_rect.min - 0.5 * indent * Vec2::X;
            let left_top = self.painter().round_pos_to_pixels(left_top);
            let left_bottom = pos2(left_top.x, child_ui.min_rect().bottom() - 2.0);
            let left_bottom = self.painter().round_pos_to_pixels(left_bottom);

            if left_vline {
                // draw a faint line on the left to mark the indented section
                self.painter.line_segment([left_top, left_bottom], stroke);
            }

            if end_with_horizontal_line {
                let fudge = 2.0; // looks nicer with button rounding in collapsing headers
                let right_bottom = pos2(child_ui.min_rect().right() - fudge, left_bottom.y);
                self.painter
                    .line_segment([left_bottom, right_bottom], stroke);
            }
        }

        let response = self.allocate_rect(child_ui.min_rect(), Sense::hover());
        InnerResponse::new(ret, response)
    }

    /// Start a ui with horizontal layout.
    /// After you have called this, the function registers the contents as any other widget.
    ///
    /// Elements will be centered on the Y axis, i.e.
    /// adjusted up and down to lie in the center of the horizontal layout.
    /// The initial height is `style.spacing.interact_size.y`.
    /// Centering is almost always what you want if you are
    /// planning to to mix widgets or use different types of text.
    ///
    /// If you don't want the contents to be centered, use [`Self::horizontal_top`] instead.
    ///
    /// The returned [`Response`] will only have checked for mouse hover
    /// but can be used for tooltips (`on_hover_text`).
    /// It also contains the [`Rect`] used by the horizontal layout.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// ui.horizontal(|ui| {
    ///     ui.label("Same");
    ///     ui.label("row");
    /// });
    /// # });
    /// ```
    ///
    /// See also [`Self::with_layout`] for more options.
    #[inline]
    pub fn horizontal<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        self.horizontal_with_main_wrap_dyn(false, Box::new(add_contents))
    }

    /// Like [`Self::horizontal`], but allocates the full vertical height and then centers elements vertically.
    pub fn horizontal_centered<R>(
        &mut self,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let initial_size = self.available_size_before_wrap();
        let layout = if self.placer.prefer_right_to_left() {
            Layout::right_to_left(Align::Center)
        } else {
            Layout::left_to_right(Align::Center)
        }
        .with_cross_align(Align::Center);
        self.allocate_ui_with_layout_dyn(initial_size, layout, Box::new(add_contents))
    }

    /// Like [`Self::horizontal`], but aligns content with top.
    pub fn horizontal_top<R>(
        &mut self,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let initial_size = self.available_size_before_wrap();
        let layout = if self.placer.prefer_right_to_left() {
            Layout::right_to_left(Align::Center)
        } else {
            Layout::left_to_right(Align::Center)
        }
        .with_cross_align(Align::Min);
        self.allocate_ui_with_layout_dyn(initial_size, layout, Box::new(add_contents))
    }

    /// Start a ui with horizontal layout that wraps to a new row
    /// when it reaches the right edge of the `max_size`.
    /// After you have called this, the function registers the contents as any other widget.
    ///
    /// Elements will be centered on the Y axis, i.e.
    /// adjusted up and down to lie in the center of the horizontal layout.
    /// The initial height is `style.spacing.interact_size.y`.
    /// Centering is almost always what you want if you are
    /// planning to to mix widgets or use different types of text.
    ///
    /// The returned [`Response`] will only have checked for mouse hover
    /// but can be used for tooltips (`on_hover_text`).
    /// It also contains the [`Rect`] used by the horizontal layout.
    ///
    /// See also [`Self::with_layout`] for more options.
    pub fn horizontal_wrapped<R>(
        &mut self,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.horizontal_with_main_wrap_dyn(true, Box::new(add_contents))
    }

    fn horizontal_with_main_wrap_dyn<'c, R>(
        &mut self,
        main_wrap: bool,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<R> {
        let initial_size = vec2(
            self.available_size_before_wrap().x,
            self.spacing().interact_size.y, // Assume there will be something interactive on the horizontal layout
        );

        let layout = if self.placer.prefer_right_to_left() {
            Layout::right_to_left(Align::Center)
        } else {
            Layout::left_to_right(Align::Center)
        }
        .with_main_wrap(main_wrap);

        self.allocate_ui_with_layout_dyn(initial_size, layout, add_contents)
    }

    /// Start a ui with vertical layout.
    /// Widgets will be left-justified.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// ui.vertical(|ui| {
    ///     ui.label("over");
    ///     ui.label("under");
    /// });
    /// # });
    /// ```
    ///
    /// See also [`Self::with_layout`] for more options.
    #[inline]
    pub fn vertical<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        self.with_layout_dyn(Layout::top_down(Align::Min), Box::new(add_contents))
    }

    /// Start a ui with vertical layout.
    /// Widgets will be horizontally centered.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// ui.vertical_centered(|ui| {
    ///     ui.label("over");
    ///     ui.label("under");
    /// });
    /// # });
    /// ```
    #[inline]
    pub fn vertical_centered<R>(
        &mut self,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.with_layout_dyn(Layout::top_down(Align::Center), Box::new(add_contents))
    }

    /// Start a ui with vertical layout.
    /// Widgets will be horizontally centered and justified (fill full width).
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// ui.vertical_centered_justified(|ui| {
    ///     ui.label("over");
    ///     ui.label("under");
    /// });
    /// # });
    /// ```
    pub fn vertical_centered_justified<R>(
        &mut self,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.with_layout_dyn(
            Layout::top_down(Align::Center).with_cross_justify(true),
            Box::new(add_contents),
        )
    }

    /// The new layout will take up all available space.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
    ///     ui.label("world!");
    ///     ui.label("Hello");
    /// });
    /// # });
    /// ```
    ///
    /// If you don't want to use up all available space, use [`Self::allocate_ui_with_layout`].
    ///
    /// See also the helpers [`Self::horizontal`], [`Self::vertical`], etc.
    #[inline]
    pub fn with_layout<R>(
        &mut self,
        layout: Layout,
        add_contents: impl FnOnce(&mut Self) -> R,
    ) -> InnerResponse<R> {
        self.with_layout_dyn(layout, Box::new(add_contents))
    }

    fn with_layout_dyn<'c, R>(
        &mut self,
        layout: Layout,
        add_contents: Box<dyn FnOnce(&mut Self) -> R + 'c>,
    ) -> InnerResponse<R> {
        let mut child_ui = self.child_ui(self.available_rect_before_wrap(), layout);
        let inner = add_contents(&mut child_ui);
        let rect = child_ui.min_rect();
        let item_spacing = self.spacing().item_spacing;
        self.placer.advance_after_rects(rect, rect, item_spacing);

        if self.style().debug.debug_on_hover && self.rect_contains_pointer(rect) {
            let painter = self.ctx().debug_painter();
            painter.rect_stroke(rect, 4.0, (1.0, Color32::LIGHT_BLUE));
            self.placer.debug_paint_cursor(&painter, "next");
        }

        InnerResponse::new(inner, self.interact(rect, child_ui.id, Sense::hover()))
    }

    #[deprecated = "Use ui.vertical_centered or ui.centered_and_justified"]
    pub fn centered<R>(&mut self, add_contents: impl FnOnce(&mut Self) -> R) -> InnerResponse<R> {
        self.vertical_centered(add_contents)
    }

    /// This will make the next added widget centered and justified in the available space.
    ///
    /// Only one widget may be added to the inner `Ui`!
    pub fn centered_and_justified<R>(
        &mut self,
        add_contents: impl FnOnce(&mut Self) -> R,
    ) -> InnerResponse<R> {
        self.with_layout_dyn(
            Layout::centered_and_justified(Direction::TopDown),
            Box::new(add_contents),
        )
    }

    pub(crate) fn set_grid(&mut self, grid: grid::GridLayout) {
        self.placer.set_grid(grid);
    }

    pub(crate) fn save_grid(&mut self) {
        self.placer.save_grid();
    }

    pub(crate) fn is_grid(&self) -> bool {
        self.placer.is_grid()
    }

    /// Move to the next row in a grid layout or wrapping layout.
    /// Otherwise does nothing.
    pub fn end_row(&mut self) {
        self.placer
            .end_row(self.spacing().item_spacing, &self.painter().clone());
    }

    /// Set row height in horizontal wrapping layout.
    pub fn set_row_height(&mut self, height: f32) {
        self.placer.set_row_height(height);
    }

    /// Temporarily split a [`Ui`] into several columns.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// ui.columns(2, |columns| {
    ///     columns[0].label("First column");
    ///     columns[1].label("Second column");
    /// });
    /// # });
    /// ```
    #[inline]
    pub fn columns<R>(
        &mut self,
        num_columns: usize,
        add_contents: impl FnOnce(&mut [Self]) -> R,
    ) -> R {
        self.columns_dyn(num_columns, Box::new(add_contents))
    }

    fn columns_dyn<'c, R>(
        &mut self,
        num_columns: usize,
        add_contents: Box<dyn FnOnce(&mut [Self]) -> R + 'c>,
    ) -> R {
        // TODO(emilk): ensure there is space
        let spacing = self.spacing().item_spacing.x;
        let total_spacing = spacing * (num_columns as f32 - 1.0);
        let column_width = (self.available_width() - total_spacing) / (num_columns as f32);
        let top_left = self.cursor().min;

        let mut columns: Vec<Self> = (0..num_columns)
            .map(|col_idx| {
                let pos = top_left + vec2((col_idx as f32) * (column_width + spacing), 0.0);
                let child_rect = Rect::from_min_max(
                    pos,
                    pos2(pos.x + column_width, self.max_rect().right_bottom().y),
                );
                let mut column_ui =
                    self.child_ui(child_rect, Layout::top_down_justified(Align::LEFT));
                column_ui.set_width(column_width);
                column_ui
            })
            .collect();

        let result = add_contents(&mut columns[..]);

        let mut max_column_width = column_width;
        let mut max_height = 0.0;
        for column in &columns {
            max_column_width = max_column_width.max(column.min_rect().width());
            max_height = column.min_size().y.max(max_height);
        }

        // Make sure we fit everything next frame:
        let total_required_width = total_spacing + max_column_width * (num_columns as f32);

        let size = vec2(self.available_width().max(total_required_width), max_height);
        self.advance_cursor_after_rect(Rect::from_min_size(top_left, size));
        result
    }

    /// Close the menu we are in (including submenus), if any.
    ///
    /// See also: [`Self::menu_button`] and [`Response::context_menu`].
    pub fn close_menu(&mut self) {
        if let Some(menu_state) = &mut self.menu_state {
            menu_state.write().close();
        }
        self.menu_state = None;
    }

    pub(crate) fn menu_state(&self) -> Option<Arc<RwLock<MenuState>>> {
        self.menu_state.clone()
    }

    pub(crate) fn set_menu_state(&mut self, menu_state: Option<Arc<RwLock<MenuState>>>) {
        self.menu_state = menu_state;
    }

    #[inline]
    /// Create a menu button that when clicked will show the given menu.
    ///
    /// If called from within a menu this will instead create a button for a sub-menu.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// ui.menu_button("My menu", |ui| {
    ///     ui.menu_button("My sub-menu", |ui| {
    ///         if ui.button("Close the menu").clicked() {
    ///             ui.close_menu();
    ///         }
    ///     });
    /// });
    /// # });
    /// ```
    ///
    /// See also: [`Self::close_menu`] and [`Response::context_menu`].
    pub fn menu_button<R>(
        &mut self,
        title: impl Into<WidgetText>,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<Option<R>> {
        if let Some(menu_state) = self.menu_state.clone() {
            menu::submenu_button(self, menu_state, title, add_contents)
        } else {
            menu::menu_button(self, title, add_contents)
        }
    }

    /// Create a menu button with an image that when clicked will show the given menu.
    ///
    /// If called from within a menu this will instead create a button for a sub-menu.
    ///
    /// ```ignore
    /// use egui_extras;
    ///
    /// let img = egui_extras::RetainedImage::from_svg_bytes_with_size(
    ///     "rss",
    ///     include_bytes!("rss.svg"),
    ///     egui_extras::image::FitTo::Size(24, 24),
    /// );
    ///
    /// ui.menu_image_button(img.texture_id(ctx), img.size_vec2(), |ui| {
    ///     ui.menu_button("My sub-menu", |ui| {
    ///         if ui.button("Close the menu").clicked() {
    ///             ui.close_menu();
    ///         }
    ///     });
    /// });
    /// ```
    ///
    /// See also: [`Self::close_menu`] and [`Response::context_menu`].
    #[inline]
    pub fn menu_image_button<R>(
        &mut self,
        texture_id: TextureId,
        image_size: impl Into<Vec2>,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<Option<R>> {
        if let Some(menu_state) = self.menu_state.clone() {
            menu::submenu_button(self, menu_state, String::new(), add_contents)
        } else {
            menu::menu_image_button(self, texture_id, image_size, add_contents)
        }
    }
}

// ----------------------------------------------------------------------------

/// # Debug stuff
impl Ui {
    /// Shows where the next widget is going to be placed
    pub fn debug_paint_cursor(&self) {
        self.placer.debug_paint_cursor(&self.painter, "next");
    }

    /// Shows the given text where the next widget is to be placed
    /// if when [`Context::set_debug_on_hover`] has been turned on and the mouse is hovering the Ui.
    pub fn trace_location(&self, text: impl ToString) {
        let rect = self.max_rect();
        if self.style().debug.debug_on_hover && self.rect_contains_pointer(rect) {
            self.placer
                .debug_paint_cursor(&self.ctx().debug_painter(), text);
        }
    }
}
