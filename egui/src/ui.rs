// #![warn(missing_docs)]

use std::{hash::Hash, sync::Arc};

use crate::{
    color::*, containers::*, layout::*, mutex::MutexGuard, paint::text::Fonts, widgets::*, *,
};

/// This is what you use to place widgets.
///
/// Represents a region of the screen with a type of layout (horizontal or vertical).
///
/// ```
/// # let mut ui = egui::Ui::__test();
/// ui.add(egui::Label::new("Hello World!"));
/// ui.label("A shorter and more convenient way to add a label.");
/// ui.horizontal(|ui| {
///     ui.label("Add widgets");
///     ui.button("on the same row!");
/// });
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
    next_auto_id: u64,

    /// Specifies paint layer, clip rectangle and a reference to `Context`.
    painter: Painter,

    /// The `Style` (visuals, spacing, etc) of this ui.
    /// Commonly many `Ui`:s share the same `Style`.
    /// The `Ui` implements copy-on-write for this.
    style: Arc<Style>,

    /// The strategy for where to put the next widget.
    layout: Layout,

    /// Sizes/bounds and cursor used by `Layout`.
    region: Region,
}

impl Ui {
    // ------------------------------------------------------------------------
    // Creation:

    pub fn new(ctx: CtxRef, layer_id: LayerId, id: Id, max_rect: Rect, clip_rect: Rect) -> Self {
        let style = ctx.style();
        let layout = Layout::default();
        let region = layout.region_from_max_rect(max_rect);
        Ui {
            id,
            next_auto_id: id.with("auto").value(),
            painter: Painter::new(ctx, layer_id, clip_rect),
            style,
            layout,
            region,
        }
    }

    pub fn child_ui(&mut self, max_rect: Rect, layout: Layout) -> Self {
        self.next_auto_id = self.next_auto_id.wrapping_add(1);
        let region = layout.region_from_max_rect(max_rect);

        Ui {
            id: self.id.with("child"),
            next_auto_id: Id::new(self.next_auto_id).with("child").value(),
            painter: self.painter.clone(),
            style: self.style.clone(),
            layout,
            region,
        }
    }

    /// Empty `Ui` for use in tests.
    pub fn __test() -> Self {
        let mut ctx = CtxRef::default();
        ctx.begin_frame(Default::default());
        let id = Id::new("__test");
        let layer_id = LayerId::new(Order::Middle, id);
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), vec2(1000.0, 1000.0));
        Self::new(ctx, layer_id, id, rect, rect)
    }

    // -------------------------------------------------

    /// A unique identity of this `Ui`.
    pub fn id(&self) -> Id {
        self.id
    }

    /// Style options for this `Ui` and its children.
    pub fn style(&self) -> &Style {
        &self.style
    }

    /// Mutably borrow internal `Style`.
    /// Changes apply to this `Ui` and its subsequent children.
    pub fn style_mut(&mut self) -> &mut Style {
        Arc::make_mut(&mut self.style) // clone-on-write
    }

    /// Changes apply to this `Ui` and its subsequent children.
    pub fn set_style(&mut self, style: impl Into<Arc<Style>>) {
        self.style = style.into();
    }

    /// Get a reference to the parent [`CtxRef`].
    pub fn ctx(&self) -> &CtxRef {
        self.painter.ctx()
    }

    /// Use this to paint stuff within this `Ui`.
    pub fn painter(&self) -> &Painter {
        &self.painter
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    /// Create a painter for a sub-region of this Ui.
    ///
    /// The clip-rect of the returned `Painter` will be the intersection
    /// of the given rectangle and the `clip_rect()` of this `Ui`.
    pub fn painter_at(&self, rect: Rect) -> Painter {
        self.painter().sub_region(rect)
    }

    /// Use this to paint stuff within this `Ui`.
    pub fn layer_id(&self) -> LayerId {
        self.painter().layer_id()
    }

    /// The `Input` of the `Context` associated with the `Ui`.
    /// Equivalent to `.ctx().input()`.
    pub fn input(&self) -> &InputState {
        self.ctx().input()
    }

    /// The `Memory` of the `Context` associated with the `Ui`.
    /// Equivalent to `.ctx().memory()`.
    pub fn memory(&self) -> MutexGuard<'_, Memory> {
        self.ctx().memory()
    }

    /// The `Output` of the `Context` associated with the `Ui`.
    /// Equivalent to `.ctx().output()`.
    pub fn output(&self) -> MutexGuard<'_, Output> {
        self.ctx().output()
    }

    /// The `Fonts` of the `Context` associated with the `Ui`.
    /// Equivalent to `.ctx().fonts()`.
    pub fn fonts(&self) -> &Fonts {
        self.ctx().fonts()
    }

    /// Screen-space rectangle for clipping what we paint in this ui.
    /// This is used, for instance, to avoid painting outside a window that is smaller than its contents.
    pub fn clip_rect(&self) -> Rect {
        self.painter.clip_rect()
    }

    /// Screen-space rectangle for clipping what we paint in this ui.
    /// This is used, for instance, to avoid painting outside a window that is smaller than its contents.
    pub fn set_clip_rect(&mut self, clip_rect: Rect) {
        self.painter.set_clip_rect(clip_rect);
    }
}

// ------------------------------------------------------------------------

/// ## Sizes etc
impl Ui {
    /// Where and how large the `Ui` is already.
    /// All widgets that have been added ot this `Ui` fits within this rectangle.
    ///
    /// No matter what, the final Ui will be at least this large.
    ///
    /// This will grow as new widgets are added, but never shrink.
    pub fn min_rect(&self) -> Rect {
        self.region.min_rect
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
    /// `Ui` will make room for it by expanding both `min_rect` and `max_rect`.
    pub fn max_rect(&self) -> Rect {
        self.region.max_rect
    }

    /// Used for animation, kind of hacky
    pub(crate) fn force_set_min_rect(&mut self, min_rect: Rect) {
        self.region.min_rect = min_rect;
    }

    /// This is like `max_rect()`, but will never be infinite.
    /// If the desired rect is infinite ("be as big as you want")
    /// this will be bounded by `min_rect` instead.
    pub fn max_rect_finite(&self) -> Rect {
        self.region.max_rect_finite()
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
        #![allow(clippy::float_cmp)]
        if self.layout.main_dir() == Direction::RightToLeft {
            debug_assert_eq!(self.min_rect().max.x, self.max_rect().max.x);
            self.region.max_rect.min.x =
                self.region.max_rect.max.x - width.at_least(self.min_rect().width());
        } else {
            debug_assert_eq!(self.min_rect().min.x, self.region.max_rect.min.x);
            self.region.max_rect.max.x =
                self.region.max_rect.min.x + width.at_least(self.min_rect().width());
        }
    }

    /// Set the maximum height of the ui.
    /// You won't be able to shrink it below the current minimum size.
    pub fn set_max_height(&mut self, height: f32) {
        #![allow(clippy::float_cmp)]
        if self.layout.main_dir() == Direction::BottomUp {
            debug_assert_eq!(self.min_rect().max.y, self.region.max_rect.max.y);
            self.region.max_rect.min.y =
                self.region.max_rect.max.y - height.at_least(self.min_rect().height());
        } else {
            debug_assert_eq!(self.min_rect().min.y, self.region.max_rect.min.y);
            self.region.max_rect.max.y =
                self.region.max_rect.min.y + height.at_least(self.min_rect().height());
        }
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
        #![allow(clippy::float_cmp)]
        if self.layout.main_dir() == Direction::RightToLeft {
            debug_assert_eq!(self.region.min_rect.max.x, self.region.max_rect.max.x);
            let min_rect = &mut self.region.min_rect;
            min_rect.min.x = min_rect.min.x.min(min_rect.max.x - width);
        } else {
            debug_assert_eq!(self.region.min_rect.min.x, self.region.max_rect.min.x);
            let min_rect = &mut self.region.min_rect;
            min_rect.max.x = min_rect.max.x.max(min_rect.min.x + width);
        }
        self.region.max_rect = self.region.max_rect.union(self.min_rect());
    }

    /// Set the minimum height of the ui.
    /// This can't shrink the ui, only make it larger.
    pub fn set_min_height(&mut self, height: f32) {
        #![allow(clippy::float_cmp)]
        if self.layout.main_dir() == Direction::BottomUp {
            debug_assert_eq!(self.region.min_rect.max.y, self.region.max_rect.max.y);
            let min_rect = &mut self.region.min_rect;
            min_rect.min.y = min_rect.min.y.min(min_rect.max.y - height);
        } else {
            debug_assert_eq!(self.region.min_rect.min.y, self.region.max_rect.min.y);
            let min_rect = &mut self.region.min_rect;
            min_rect.max.y = min_rect.max.y.max(min_rect.min.y + height);
        }
        self.region.max_rect = self.region.max_rect.union(self.min_rect());
    }

    // ------------------------------------------------------------------------

    /// Helper: shrinks the max width to the current width,
    /// so further widgets will try not to be wider than previous widgets.
    /// Useful for normal vertical layouts.
    pub fn shrink_width_to_current(&mut self) {
        self.set_max_width(self.min_rect().width())
    }

    /// Helper: shrinks the max height to the current height,
    /// so further widgets will try not to be wider than previous widgets.
    pub fn shrink_height_to_current(&mut self) {
        self.set_max_height(self.min_rect().height())
    }

    /// Expand the `min_rect` and `max_rect` of this ui to include a child at the given rect.
    pub fn expand_to_include_rect(&mut self, rect: Rect) {
        self.region.expand_to_include_rect(rect);
    }

    /// `ui.set_width_range(min..=max);` is equivalent to `ui.set_min_width(min); ui.set_max_width(max);`.
    pub fn set_width_range(&mut self, width: std::ops::RangeInclusive<f32>) {
        self.set_min_width(*width.start());
        self.set_max_width(*width.end());
    }

    /// `ui.set_width_range(width);` is equivalent to `ui.set_min_width(width); ui.set_max_width(width);`.
    pub fn set_width(&mut self, width: f32) {
        self.set_min_width(width);
        self.set_max_width(width);
    }

    // ------------------------------------------------------------------------
    // Layout related measures:

    /// The available space at the moment, given the current cursor.
    /// This how much more space we can take up without overflowing our parent.
    /// Shrinks as widgets allocate space and the cursor moves.
    /// A small size should be interpreted as "as little as possible".
    /// An infinite size should be interpreted as "as much as you want".
    pub fn available_size(&self) -> Vec2 {
        self.layout.available_size(&self.region)
    }

    pub fn available_width(&self) -> f32 {
        self.available_size().x
    }

    /// In case of a wrapping layout, how much space is left on this row/column?
    pub fn available_size_before_wrap(&self) -> Vec2 {
        self.layout.available_rect_before_wrap(&self.region).size()
    }

    /// This is like `available_size_before_wrap()`, but will never be infinite.
    /// Use this for components that want to grow without bounds (but shouldn't).
    /// In most layouts the next widget will be put in the top left corner of this `Rect`.
    pub fn available_size_before_wrap_finite(&self) -> Vec2 {
        self.layout
            .available_rect_before_wrap_finite(&self.region)
            .size()
    }

    pub fn available_rect_before_wrap(&self) -> Rect {
        self.layout.available_rect_before_wrap(&self.region)
    }

    /// This is like `available_rect_before_wrap()`, but will never be infinite.
    /// Use this for components that want to grow without bounds (but shouldn't).
    /// In most layouts the next widget will be put in the top left corner of this `Rect`.
    pub fn available_rect_before_wrap_finite(&self) -> Rect {
        self.layout.available_rect_before_wrap_finite(&self.region)
    }
}

/// # `Id` creation
impl Ui {
    /// Use this to generate widget ids for widgets that have persistent state in `Memory`.
    pub fn make_persistent_id<IdSource>(&self, id_source: IdSource) -> Id
    where
        IdSource: Hash + std::fmt::Debug,
    {
        self.id.with(&id_source)
    }

    #[deprecated = "This id now returned from ui.allocate_space"]
    pub fn make_position_id(&self) -> Id {
        Id::new(self.next_auto_id)
    }

    pub(crate) fn auto_id_with<IdSource>(&self, id_source: IdSource) -> Id
    where
        IdSource: Hash + std::fmt::Debug,
    {
        Id::new(self.next_auto_id).with(id_source)
    }
}

/// # Interaction
impl Ui {
    pub fn interact(&self, rect: Rect, id: Id, sense: Sense) -> Response {
        self.ctx().interact(
            self.clip_rect(),
            self.style().spacing.item_spacing,
            self.layer_id(),
            id,
            rect,
            sense,
        )
    }

    pub fn rect_contains_mouse(&self, rect: Rect) -> bool {
        self.ctx()
            .rect_contains_mouse(self.layer_id(), self.clip_rect().intersect(rect))
    }

    /// Is the mouse above this `Ui`?
    /// Equivalent to `ui.rect_contains_mouse(ui.min_rect())`
    pub fn ui_contains_mouse(&self) -> bool {
        self.rect_contains_mouse(self.min_rect())
    }

    #[deprecated = "Use: interact(rect, id, Sense::hover())"]
    pub fn interact_hover(&self, rect: Rect) -> Response {
        self.interact(rect, self.auto_id_with("hover_rect"), Sense::hover())
    }

    #[deprecated = "Use: rect_contains_mouse()"]
    pub fn hovered(&self, rect: Rect) -> bool {
        self.interact(rect, self.id, Sense::hover()).hovered
    }

    // ------------------------------------------------------------------------
    // Stuff that moves the cursor, i.e. allocates space in this ui!

    /// Advance the cursor (where the next widget is put) by this many points.
    /// The direction is dependent on the layout.
    /// This is useful for creating some extra space between widgets.
    pub fn advance_cursor(&mut self, amount: f32) {
        self.layout.advance_cursor(&mut self.region, amount);
    }

    /// Allocate space for a widget and check for interaction in the space.
    /// Returns a `Response` which contains a rectangle, id, and interaction info.
    ///
    /// ## How sizes are negotiated
    /// Each widget should have a *minimum desired size* and a *desired size*.
    /// When asking for space, ask AT LEAST for you minimum, and don't ask for more than you need.
    /// If you want to fill the space, ask about `available().size()` and use that.
    ///
    /// You may get MORE space than you asked for, for instance
    /// for justified layouts, like in menus.
    ///
    /// You will never get a rectangle that is smaller than the amount of space you asked for.
    ///
    /// ```
    /// # let mut ui = egui::Ui::__test();
    /// let response = ui.allocate_response(egui::vec2(100.0, 200.0), egui::Sense::click());
    /// if response.clicked { /* … */ }
    /// ui.painter().rect_stroke(response.rect, 0.0, (1.0, egui::Color32::WHITE));
    /// ```
    pub fn allocate_response(&mut self, desired_size: Vec2, sense: Sense) -> Response {
        let (id, rect) = self.allocate_space(desired_size);
        self.interact(rect, id, sense)
    }

    /// Returns a `Rect` with exactly what you asked for.
    ///
    /// The response rect will be larger if this is part of a justified layout or similar.
    /// This means that iof this is a narrow widget in a wide justified layout, then
    /// the widget will react to interactions outside the returned `Rect`.
    pub fn allocate_exact_size(&mut self, desired_size: Vec2, sense: Sense) -> (Rect, Response) {
        let response = self.allocate_response(desired_size, sense);
        let rect = self
            .layout()
            .align_size_within_rect(desired_size, response.rect);
        (rect, response)
    }

    /// Allocate at least as much space as needed, and interact with that rect.
    ///
    /// The returned `Rect` will be the same size as `Response::rect`.
    pub fn allocate_at_least(&mut self, desired_size: Vec2, sense: Sense) -> (Rect, Response) {
        let response = self.allocate_response(desired_size, sense);
        (response.rect, response)
    }

    /// Reserve this much space and move the cursor.
    /// Returns where to put the widget.
    ///
    /// ## How sizes are negotiated
    /// Each widget should have a *minimum desired size* and a *desired size*.
    /// When asking for space, ask AT LEAST for you minimum, and don't ask for more than you need.
    /// If you want to fill the space, ask about `available().size()` and use that.
    ///
    /// You may get MORE space than you asked for, for instance
    /// for justified layouts, like in menus.
    ///
    /// You will never get a rectangle that is smaller than the amount of space you asked for.
    ///
    /// Returns an automatic `Id` (which you can use for interaction) and the `Rect` of where to put your widget.
    ///
    /// ```
    /// # let mut ui = egui::Ui::__test();
    /// let (id, rect) = ui.allocate_space(egui::vec2(100.0, 200.0));
    /// let response = ui.interact(rect, id, egui::Sense::click());
    /// ```
    pub fn allocate_space(&mut self, desired_size: Vec2) -> (Id, Rect) {
        // For debug rendering
        let original_available = self.available_size_before_wrap();
        let too_wide = desired_size.x > original_available.x;
        let too_high = desired_size.y > original_available.y;

        let rect = self.allocate_space_impl(desired_size);

        let debug_expand_width = self.style().visuals.debug_expand_width;
        let debug_expand_height = self.style().visuals.debug_expand_height;

        if (debug_expand_width && too_wide) || (debug_expand_height && too_high) {
            self.painter
                .rect_stroke(rect, 0.0, (1.0, Color32::LIGHT_BLUE));

            let color = color::Color32::from_rgb(200, 0, 0);
            let width = 2.5;

            let paint_line_seg = |a, b| self.painter().line_segment([a, b], (width, color));

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

        self.next_auto_id = self.next_auto_id.wrapping_add(1);
        let id = Id::new(self.next_auto_id);

        (id, rect)
    }

    /// Reserve this much space and move the cursor.
    /// Returns where to put the widget.
    fn allocate_space_impl(&mut self, desired_size: Vec2) -> Rect {
        let item_spacing = self.style().spacing.item_spacing;
        let outer_child_rect = self
            .layout
            .next_space(&self.region, desired_size, item_spacing);
        let inner_child_rect = self.layout.justify_or_align(outer_child_rect, desired_size);

        self.layout.advance_after_outer_rect(
            &mut self.region,
            outer_child_rect,
            inner_child_rect,
            item_spacing,
        );
        self.region.expand_to_include_rect(inner_child_rect);

        inner_child_rect
    }

    pub(crate) fn advance_cursor_after_rect(&mut self, rect: Rect) -> Id {
        let item_spacing = self.style().spacing.item_spacing;
        self.layout
            .advance_after_outer_rect(&mut self.region, rect, rect, item_spacing);
        self.region.expand_to_include_rect(rect);

        self.next_auto_id = self.next_auto_id.wrapping_add(1);
        Id::new(self.next_auto_id)
    }

    pub(crate) fn cursor(&self) -> Pos2 {
        self.region.cursor
    }

    /// Allocated the given space and then adds content to that space.
    /// If the contents overflow, more space will be allocated.
    /// When finished, the amount of space actually used (`min_rect`) will be allocated.
    /// So you can request a lot of space and then use less.
    pub fn allocate_ui<R>(
        &mut self,
        desired_size: Vec2,
        add_contents: impl FnOnce(&mut Self) -> R,
    ) -> (R, Response) {
        let item_spacing = self.style().spacing.item_spacing;
        let outer_child_rect = self
            .layout
            .next_space(&self.region, desired_size, item_spacing);
        let inner_child_rect = self.layout.justify_or_align(outer_child_rect, desired_size);

        let mut child_ui = self.child_ui(inner_child_rect, self.layout);
        let ret = add_contents(&mut child_ui);
        let final_child_rect = child_ui.region.min_rect;

        self.layout.advance_after_outer_rect(
            &mut self.region,
            outer_child_rect.union(final_child_rect),
            final_child_rect,
            item_spacing,
        );
        self.region.expand_to_include_rect(final_child_rect);

        let response = self.interact(final_child_rect, child_ui.id, Sense::hover());
        (ret, response)
    }

    /// Convenience function to get a region to paint on
    pub fn allocate_painter(&mut self, desired_size: Vec2, sense: Sense) -> (Response, Painter) {
        let response = self.allocate_response(desired_size, sense);
        let clip_rect = self.clip_rect().intersect(response.rect); // Make sure we don't paint out of bounds
        let painter = Painter::new(self.ctx().clone(), self.layer_id(), clip_rect);
        (response, painter)
    }

    /// Move the scroll to this cursor position with the specified alignment.
    ///
    /// ```
    /// # use egui::Align;
    /// # let mut ui = &mut egui::Ui::__test();
    /// egui::ScrollArea::auto_sized().show(ui, |ui| {
    ///     let scroll_bottom = ui.button("Scroll to bottom.").clicked;
    ///     for i in 0..1000 {
    ///         ui.label(format!("Item {}", i));
    ///     }
    ///
    ///     if scroll_bottom {
    ///         ui.scroll_to_cursor(Align::bottom());
    ///     }
    /// });
    /// ```
    pub fn scroll_to_cursor(&mut self, align: Align) {
        let scroll_y = self.region.cursor.y;

        self.ctx().frame_state().scroll_target = Some((scroll_y, align));
    }
}

/// # Adding widgets
impl Ui {
    /// Add a widget to this `Ui` at a location dependent on the current [`Layout`].
    ///
    /// The returned [`Response`] can be used to check for interactions,
    /// as well as adding tooltips using [`Response::on_hover_text`].
    ///
    /// ```
    /// # let mut ui = egui::Ui::__test();
    /// # let mut my_value = 42;
    /// let response = ui.add(egui::Slider::i32(&mut my_value, 0..=100));
    /// response.on_hover_text("Drag me!");
    /// ```
    pub fn add(&mut self, widget: impl Widget) -> Response {
        widget.ui(self)
    }

    /// Shortcut for `add(Label::new(text))`
    pub fn label(&mut self, label: impl Into<Label>) -> Response {
        self.add(label.into())
    }

    /// Shortcut for `add(Label::new(text).text_color(color))`
    pub fn colored_label(
        &mut self,
        color: impl Into<Color32>,
        label: impl Into<Label>,
    ) -> Response {
        self.add(label.into().text_color(color))
    }

    /// Shortcut for `add(Label::new(text).heading())`
    pub fn heading(&mut self, label: impl Into<Label>) -> Response {
        self.add(label.into().heading())
    }

    /// Shortcut for `add(Label::new(text).monospace())`
    pub fn monospace(&mut self, label: impl Into<Label>) -> Response {
        self.add(label.into().monospace())
    }

    /// Shortcut for `add(Label::new(text).small())`
    pub fn small(&mut self, label: impl Into<Label>) -> Response {
        self.add(label.into().small())
    }

    /// Shortcut for `add(Hyperlink::new(url))`
    pub fn hyperlink(&mut self, url: impl Into<String>) -> Response {
        self.add(Hyperlink::new(url))
    }

    #[deprecated = "Use `text_edit_singleline` or `text_edit_multiline`"]
    pub fn text_edit(&mut self, text: &mut String) -> Response {
        self.text_edit_multiline(text)
    }

    /// Now newlines (`\n`) allowed. Pressing enter key will result in the `TextEdit` loosing focus (`response.lost_kb_focus`).
    pub fn text_edit_singleline(&mut self, text: &mut String) -> Response {
        self.add(TextEdit::singleline(text))
    }

    /// A `TextEdit` for multiple lines. Pressing enter key will create a new line.
    pub fn text_edit_multiline(&mut self, text: &mut String) -> Response {
        self.add(TextEdit::multiline(text))
    }

    /// Usage: `if ui.button("Click me").clicked { ... }`
    ///
    /// Shortcut for `add(Button::new(text))`
    #[must_use = "You should check if the user clicked this with `if ui.button(...).clicked { ... } "]
    pub fn button(&mut self, text: impl Into<String>) -> Response {
        self.add(Button::new(text))
    }

    /// A button as small as normal body text.
    ///
    /// Usage: `if ui.small_button("Click me").clicked { ... }`
    ///
    /// Shortcut for `add(Button::new(text).small())`
    #[must_use = "You should check if the user clicked this with `if ui.small_button(...).clicked { ... } "]
    pub fn small_button(&mut self, text: impl Into<String>) -> Response {
        self.add(Button::new(text).small())
    }

    /// Show a checkbox.
    pub fn checkbox(&mut self, checked: &mut bool, text: impl Into<String>) -> Response {
        self.add(Checkbox::new(checked, text))
    }

    /// Show a radio button.
    /// Often you want to use `ui.radio_value` instead.
    pub fn radio(&mut self, selected: bool, text: impl Into<String>) -> Response {
        self.add(RadioButton::new(selected, text))
    }

    /// Show a radio button. It is selected if `*current_value == selected_value`.
    /// If clicked, `selected_value` is assigned to `*current_value`.
    ///
    /// Example: `ui.radio_value(&mut my_enum, Enum::Alternative, "Alternative")`.
    pub fn radio_value<Value: PartialEq>(
        &mut self,
        current_value: &mut Value,
        selected_value: Value,
        text: impl Into<String>,
    ) -> Response {
        let response = self.radio(*current_value == selected_value, text);
        if response.clicked {
            *current_value = selected_value;
        }
        response
    }

    /// Show a label which can be selected or not.
    pub fn selectable_label(&mut self, checked: bool, text: impl Into<String>) -> Response {
        self.add(SelectableLabel::new(checked, text))
    }

    /// Show selectable text. It is selected if `*current_value == selected_value`.
    /// If clicked, `selected_value` is assigned to `*current_value`.
    ///
    /// Example: `ui.selectable_value(&mut my_enum, Enum::Alternative, "Alternative")`.
    pub fn selectable_value<Value: PartialEq>(
        &mut self,
        current_value: &mut Value,
        selected_value: Value,
        text: impl Into<String>,
    ) -> Response {
        let response = self.selectable_label(*current_value == selected_value, text);
        if response.clicked {
            *current_value = selected_value;
        }
        response
    }

    /// Shortcut for `add(Separator::new())`
    pub fn separator(&mut self) -> Response {
        self.add(Separator::new())
    }

    /// Modify an angle. The given angle should be in radians, but is shown to the user in degrees.
    /// The angle is NOT wrapped, so the user may select, for instance 720° = 2𝞃 = 4π
    pub fn drag_angle(&mut self, radians: &mut f32) -> Response {
        #![allow(clippy::float_cmp)]

        let mut degrees = radians.to_degrees();
        let response = self.add(DragValue::f32(&mut degrees).speed(1.0).suffix("°"));

        // only touch `*radians` if we actually changed the degree value
        if degrees != radians.to_degrees() {
            *radians = degrees.to_radians();
        }

        response
    }

    /// Modify an angle. The given angle should be in radians,
    /// but is shown to the user in fractions of one Tau (i.e. fractions of one turn).
    /// The angle is NOT wrapped, so the user may select, for instance 2𝞃 (720°)
    pub fn drag_angle_tau(&mut self, radians: &mut f32) -> Response {
        #![allow(clippy::float_cmp)]

        use std::f32::consts::TAU;

        let mut taus = *radians / TAU;
        let response = self
            .add(DragValue::f32(&mut taus).speed(0.01).suffix("τ"))
            .on_hover_text("1τ = one turn, 0.5τ = half a turn, etc. 0.25τ = 90°");

        // only touch `*radians` if we actually changed the value
        if taus != *radians / TAU {
            *radians = taus * TAU;
        }

        response
    }

    /// Show an image here with the given size
    pub fn image(&mut self, texture_id: TextureId, desired_size: impl Into<Vec2>) -> Response {
        self.add(Image::new(texture_id, desired_size))
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
        let mut hsva = Hsva::from_srgb(*srgb);
        let response =
            color_picker::color_edit_button_hsva(self, &mut hsva, color_picker::Alpha::Opaque);
        *srgb = hsva.to_srgb();
        response
    }

    /// Shows a button with the given color.
    /// If the user clicks the button, a full color picker is shown.
    /// The given color is in linear RGB space.
    pub fn color_edit_button_rgb(&mut self, rgb: &mut [f32; 3]) -> Response {
        let mut hsva = Hsva::from_rgb(*rgb);
        let response =
            color_picker::color_edit_button_hsva(self, &mut hsva, color_picker::Alpha::Opaque);
        *rgb = hsva.to_rgb();
        response
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
        let mut hsva = Hsva::from_srgba_unmultiplied(*srgba);
        let response =
            color_picker::color_edit_button_hsva(self, &mut hsva, color_picker::Alpha::OnlyBlend);
        *srgba = hsva.to_srgba_unmultiplied();
        response
    }

    /// Shows a button with the given color.
    /// If the user clicks the button, a full color picker is shown.
    /// The given color is in linear RGBA space with premultiplied alpha
    pub fn color_edit_button_rgba_premultiplied(&mut self, rgba: &mut [f32; 4]) -> Response {
        let mut hsva = Hsva::from_rgba_premultiplied(*rgba);
        let response = color_picker::color_edit_button_hsva(
            self,
            &mut hsva,
            color_picker::Alpha::BlendOrAdditive,
        );
        *rgba = hsva.to_rgba_premultiplied();
        response
    }

    /// Shows a button with the given color.
    /// If the user clicks the button, a full color picker is shown.
    /// The given color is in linear RGBA space without premultiplied alpha.
    /// If unsure, what "premultiplied alpha" is, then this is probably the function you want to use.
    pub fn color_edit_button_rgba_unmultiplied(&mut self, rgba: &mut [f32; 4]) -> Response {
        let mut hsva = Hsva::from_rgba_unmultiplied(*rgba);
        let response =
            color_picker::color_edit_button_hsva(self, &mut hsva, color_picker::Alpha::OnlyBlend);
        *rgba = hsva.to_rgba_unmultiplied();
        response
    }
}

/// # Adding Containers / Sub-uis:
impl Ui {
    /// Create a child ui. You can use this to temporarily change the Style of a sub-region, for instance.
    pub fn wrap<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> (R, Response) {
        let child_rect = self.available_rect_before_wrap();
        let mut child_ui = self.child_ui(child_rect, self.layout);
        let ret = add_contents(&mut child_ui);
        let size = child_ui.min_size();
        let response = self.allocate_response(size, Sense::hover());
        (ret, response)
    }

    /// Redirect shapes to another paint layer.
    pub fn with_layer_id<R>(
        &mut self,
        layer_id: LayerId,
        add_contents: impl FnOnce(&mut Self) -> R,
    ) -> (R, Response) {
        self.wrap(|ui| {
            ui.painter.set_layer_id(layer_id);
            add_contents(ui)
        })
    }

    #[deprecated = "Use `ui.allocate_ui` instead"]
    pub fn add_custom_contents(
        &mut self,
        desired_size: Vec2,
        add_contents: impl FnOnce(&mut Ui),
    ) -> Rect {
        self.allocate_ui(desired_size, add_contents).1.rect
    }

    /// A `CollapsingHeader` that starts out collapsed.
    pub fn collapsing<R>(
        &mut self,
        heading: impl Into<String>,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> CollapsingResponse<R> {
        CollapsingHeader::new(heading).show(self, add_contents)
    }

    /// Create a child ui which is indented to the right
    pub fn indent<R>(
        &mut self,
        id_source: impl Hash,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> (R, Response) {
        assert!(
            self.layout.is_vertical(),
            "You can only indent vertical layouts, found {:?}",
            self.layout
        );
        let indent = vec2(self.style().spacing.indent, 0.0);
        let child_rect =
            Rect::from_min_max(self.region.cursor + indent, self.max_rect().right_bottom()); // TODO: wrong for reversed layouts
        let mut child_ui = Self {
            id: self.id.with(id_source),
            ..self.child_ui(child_rect, self.layout)
        };
        let ret = add_contents(&mut child_ui);
        let size = child_ui.min_size();

        // draw a grey line on the left to mark the indented section
        let line_start = child_rect.min - indent * 0.5;
        let line_start = self.painter().round_pos_to_pixels(line_start);
        let line_end = pos2(line_start.x, line_start.y + size.y - 2.0);
        self.painter.line_segment(
            [line_start, line_end],
            self.style().visuals.widgets.noninteractive.bg_stroke,
        );

        let response = self.allocate_response(indent + size, Sense::hover());
        (ret, response)
    }

    #[deprecated]
    pub fn left_column(&mut self, width: f32) -> Self {
        #[allow(deprecated)]
        self.column(Align::Min, width)
    }

    #[deprecated]
    pub fn centered_column(&mut self, width: f32) -> Self {
        #[allow(deprecated)]
        self.column(Align::Center, width)
    }

    #[deprecated]
    pub fn right_column(&mut self, width: f32) -> Self {
        #[allow(deprecated)]
        self.column(Align::Max, width)
    }

    /// A column ui with a given width.
    #[deprecated]
    pub fn column(&mut self, column_position: Align, width: f32) -> Self {
        let x = match column_position {
            Align::Min => 0.0,
            Align::Center => self.available_width() / 2.0 - width / 2.0,
            Align::Max => self.available_width() - width,
        };
        self.child_ui(
            Rect::from_min_size(
                self.region.cursor + vec2(x, 0.0),
                vec2(width, self.available_size_before_wrap().y),
            ),
            self.layout,
        )
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
    /// The returned `Response` will only have checked for mouse hover
    /// but can be used for tooltips (`on_hover_text`).
    /// It also contains the `Rect` used by the horizontal layout.
    pub fn horizontal<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> (R, Response) {
        self.horizontal_with_main_wrap(false, add_contents)
    }

    /// Like `horizontal`, but will set up the spacing to match that of a normal label.
    ///
    /// In particular, the space between widgets is the same width as the space character.
    ///
    /// You can still add any widgets to the layout (not only Labels).
    pub fn horizontal_for_text<R>(
        &mut self,
        text_style: TextStyle,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> (R, Response) {
        self.wrap(|ui| {
            let font = &ui.fonts()[text_style];
            let row_height = font.row_height();
            let space_width = font.glyph_width(' ');
            let style = ui.style_mut();
            style.spacing.interact_size.y = row_height;
            style.spacing.item_spacing.x = space_width;
            style.spacing.item_spacing.y = 0.0;
            ui.horizontal(add_contents).0
        })
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
    /// The returned `Response` will only have checked for mouse hover
    /// but can be used for tooltips (`on_hover_text`).
    /// It also contains the `Rect` used by the horizontal layout.
    pub fn horizontal_wrapped<R>(
        &mut self,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> (R, Response) {
        self.horizontal_with_main_wrap(true, add_contents)
    }

    /// Like `horizontal_wrapped`, but will set up the spacing and
    /// line size to match that of a normal label.
    ///
    /// In particular, the space between widgets is the same width as the space character
    /// and the line spacing is the same as that for text.
    ///
    /// You can still add any widgets to the layout (not only Labels).
    pub fn horizontal_wrapped_for_text<R>(
        &mut self,
        text_style: TextStyle,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> (R, Response) {
        self.wrap(|ui| {
            let font = &ui.fonts()[text_style];
            let row_height = font.row_height();
            let space_width = font.glyph_width(' ');
            let style = ui.style_mut();
            style.spacing.interact_size.y = row_height;
            style.spacing.item_spacing.x = space_width;
            style.spacing.item_spacing.y = 0.0;
            ui.horizontal_wrapped(add_contents).0
        })
    }

    fn horizontal_with_main_wrap<R>(
        &mut self,
        main_wrap: bool,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> (R, Response) {
        let initial_size = vec2(
            self.available_size_before_wrap_finite().x,
            self.style().spacing.interact_size.y, // Assume there will be something interactive on the horizontal layout
        );

        let layout = if self.layout.prefer_right_to_left() {
            Layout::right_to_left()
        } else {
            Layout::left_to_right()
        }
        .with_main_wrap(main_wrap);

        self.allocate_ui(initial_size, |ui| ui.with_layout(layout, add_contents).0)
    }

    /// Start a ui with vertical layout.
    /// Widgets will be left-justified.
    pub fn vertical<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> (R, Response) {
        self.with_layout(Layout::top_down(Align::Min), add_contents)
    }

    /// Start a ui with vertical layout.
    /// Widgets will be centered.
    pub fn vertical_centered<R>(
        &mut self,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> (R, Response) {
        self.with_layout(Layout::top_down(Align::Center), add_contents)
    }
    /// Start a ui with vertical layout.
    /// Widgets will be centered and justified (fill full width).
    pub fn vertical_centered_justified<R>(
        &mut self,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> (R, Response) {
        self.with_layout(
            Layout::top_down(Align::Center).with_cross_justify(true),
            add_contents,
        )
    }

    pub fn with_layout<R>(
        &mut self,
        layout: Layout,
        add_contents: impl FnOnce(&mut Self) -> R,
    ) -> (R, Response) {
        let mut child_ui = self.child_ui(self.available_rect_before_wrap(), layout);
        let ret = add_contents(&mut child_ui);
        let rect = child_ui.min_rect();
        let item_spacing = self.style().spacing.item_spacing;
        self.layout
            .advance_after_outer_rect(&mut self.region, rect, rect, item_spacing);
        self.region.expand_to_include_rect(rect);
        (ret, self.interact(rect, child_ui.id, Sense::hover()))
    }

    /// Temporarily split split an Ui into several columns.
    ///
    /// ```
    /// # let mut ui = egui::Ui::__test();
    /// ui.columns(2, |columns| {
    ///     columns[0].label("First column");
    ///     columns[1].label("Second column");
    /// });
    /// ```
    pub fn columns<F, R>(&mut self, num_columns: usize, add_contents: F) -> R
    where
        F: FnOnce(&mut [Self]) -> R,
    {
        // TODO: ensure there is space
        let spacing = self.style().spacing.item_spacing.x;
        let total_spacing = spacing * (num_columns as f32 - 1.0);
        let column_width = (self.available_width() - total_spacing) / (num_columns as f32);
        let top_left = self.region.cursor;

        let mut columns: Vec<Self> = (0..num_columns)
            .map(|col_idx| {
                let pos = top_left + vec2((col_idx as f32) * (column_width + spacing), 0.0);
                let child_rect = Rect::from_min_max(
                    pos,
                    pos2(pos.x + column_width, self.max_rect().right_bottom().y),
                );
                let mut column_ui =
                    self.child_ui(child_rect, Layout::top_down_justified(Align::left()));
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
}

// ----------------------------------------------------------------------------

/// ## Debug stuff
impl Ui {
    /// Shows where the next widget is going to be placed
    pub fn debug_paint_cursor(&self) {
        self.layout.debug_paint_cursor(&self.region, &self.painter);
    }
}
