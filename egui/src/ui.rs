#![allow(clippy::float_cmp)]

use std::{hash::Hash, sync::Arc};

use crate::{color::*, containers::*, layout::*, paint::*, widgets::*, *};

/// Represents a region of the screen
/// with a type of layout (horizontal or vertical).
pub struct Ui {
    /// ID of this ui.
    /// Generated based on id of parent ui together with
    /// another source of child identity (e.g. window title).
    /// Acts like a namespace for child uis.
    /// Hopefully unique.
    id: Id,

    painter: Painter,

    /// This is the minimal size of the `Ui`.
    /// When adding new widgets, this will generally expand.
    ///
    /// Always finite.
    ///
    /// The bounding box of all child widgets, but not necessarily a tight bounding box
    /// since `Ui` can start with a non-zero min_rect size.
    min_rect: Rect,

    /// The maximum size of this `Ui`. This is a *soft max*
    /// meaning new widgets will *try* not to expand beyond it,
    /// but if they have to, they will.
    ///
    /// Text will wrap at `max_rect.right()`.
    /// Some widgets (like separator lines) will try to fill the full `max_rect` width of the ui.
    ///
    /// `max_rect` will always be at least the size of `min_rect`.
    ///
    /// If the `max_rect` size is zero, it is a signal that child widgets should be as small as possible.
    /// If the `max_rect` size is infinite, it is a signal that child widgets should take up as much room as they want.
    max_rect: Rect,

    /// Override default style in this ui
    style: Style,

    layout: Layout,

    /// Where the next widget will be put.
    /// Progresses along self.dir.
    /// Initially set to rect.min
    /// If something has already been added, this will point ot style.spacing.item_spacing beyond the latest child.
    /// The cursor can thus be style.spacing.item_spacing pixels outside of the min_rect.
    cursor: Pos2, // TODO: move into Layout?

    /// How many children has been added to us?
    /// This is only used to create a unique interact ID for some widgets
    /// that work as long as no other widgets are added/removed while interacting.
    child_count: usize,
}

impl Ui {
    // ------------------------------------------------------------------------
    // Creation:

    pub fn new(ctx: Arc<Context>, layer: Layer, id: Id, max_rect: Rect) -> Self {
        let style = ctx.style();
        let clip_rect = max_rect.expand(style.visuals.clip_rect_margin);
        let layout = Layout::default();
        let cursor = layout.initial_cursor(max_rect);
        let min_size = Vec2::zero(); // TODO: From Style
        let min_rect = layout.rect_from_cursor_size(cursor, min_size);
        Ui {
            id,
            painter: Painter::new(ctx, layer, clip_rect),
            min_rect,
            max_rect,
            style,
            layout,
            cursor,
            child_count: 0,
        }
    }

    pub fn child_ui(&mut self, max_rect: Rect, layout: Layout) -> Self {
        let id = self.make_position_id(); // TODO: is this a good idea?
        self.child_count += 1;

        let cursor = layout.initial_cursor(max_rect);
        let min_size = Vec2::zero(); // TODO: From Style
        let min_rect = layout.rect_from_cursor_size(cursor, min_size);

        Ui {
            id,
            painter: self.painter.clone(),
            min_rect,
            max_rect,
            style: self.style().clone(),
            layout,
            cursor,
            child_count: 0,
        }
    }

    // -------------------------------------------------

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
        &mut self.style
    }

    pub fn set_style(&mut self, style: Style) {
        self.style = style
    }

    pub fn ctx(&self) -> &Arc<Context> {
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
    pub fn layer(&self) -> Layer {
        self.painter().layer()
    }

    /// The `Input` of the `Context` associated with the `Ui`.
    /// Equivalent to `.ctx().input()`.
    pub fn input(&self) -> &InputState {
        self.ctx().input()
    }

    /// The `Memory` of the `Context` associated with the `Ui`.
    /// Equivalent to `.ctx().memory()`.
    pub fn memory(&self) -> parking_lot::MutexGuard<'_, Memory> {
        self.ctx().memory()
    }

    /// The `Output` of the `Context` associated with the `Ui`.
    /// Equivalent to `.ctx().output()`.
    pub fn output(&self) -> parking_lot::MutexGuard<'_, Output> {
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
    /// The current size of this Ui.
    /// Bounding box of all contained child widgets.
    /// No matter what, the final Ui will be at least this large.
    /// This will grow as new widgets are added, but never shrink.
    pub fn min_rect(&self) -> Rect {
        self.min_rect
    }

    /// Size of content; same as `min_rect().size()`
    pub fn min_size(&self) -> Vec2 {
        self.min_rect.size()
    }

    /// This is the soft max size of the Ui.
    /// New widgets will *try* to fit within this rectangle.
    /// For instance, text will wrap to fit within it.
    /// If a widget doesn't fit within the `max_rect` then it will expand.
    /// `max_rect()` is always at least as large as `min_rect()`.
    pub fn max_rect(&self) -> Rect {
        self.max_rect
    }

    /// Used for animation, kind of hacky
    pub(crate) fn force_set_min_rect(&mut self, min_rect: Rect) {
        self.min_rect = min_rect;
    }

    /// This is like `max_rect()`, but will never be infinite.
    /// If the desired rect is infinite ("be as big as you want")
    /// this will be bounded by `min_rect` instead.
    pub fn max_rect_finite(&self) -> Rect {
        let mut result = self.max_rect;
        if !result.min.x.is_finite() {
            result.min.x = self.min_rect.min.x;
        }
        if !result.min.y.is_finite() {
            result.min.y = self.min_rect.min.y;
        }
        if !result.max.x.is_finite() {
            result.max.x = self.min_rect.max.x;
        }
        if !result.max.y.is_finite() {
            result.max.y = self.min_rect.max.y;
        }
        result
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
        if self.layout.dir() == Direction::Horizontal && self.layout.is_reversed() {
            debug_assert_eq!(self.min_rect.max.x, self.max_rect.max.x);
            self.max_rect.min.x = self.max_rect.max.x - width.at_least(self.min_rect.width());
        } else {
            debug_assert_eq!(self.min_rect.min.x, self.max_rect.min.x);
            self.max_rect.max.x = self.max_rect.min.x + width.at_least(self.min_rect.width());
        }
    }

    /// Set the maximum height of the ui.
    /// You won't be able to shrink it below the current minimum size.
    pub fn set_max_height(&mut self, height: f32) {
        if self.layout.dir() == Direction::Vertical && self.layout.is_reversed() {
            debug_assert_eq!(self.min_rect.max.y, self.max_rect.max.y);
            self.max_rect.min.y = self.max_rect.max.y - height.at_least(self.min_rect.height());
        } else {
            debug_assert_eq!(self.min_rect.min.y, self.max_rect.min.y);
            self.max_rect.max.y = self.max_rect.min.y + height.at_least(self.min_rect.height());
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
        if self.layout.dir() == Direction::Horizontal && self.layout.is_reversed() {
            debug_assert_eq!(self.min_rect.max.x, self.max_rect.max.x);
            self.min_rect.min.x = self.min_rect.min.x.min(self.min_rect.max.x - width);
        } else {
            debug_assert_eq!(self.min_rect.min.x, self.max_rect.min.x);
            self.min_rect.max.x = self.min_rect.max.x.max(self.min_rect.min.x + width);
        }
        self.max_rect = self.max_rect.union(self.min_rect);
    }

    /// Set the minimum height of the ui.
    /// This can't shrink the ui, only make it larger.
    pub fn set_min_height(&mut self, height: f32) {
        if self.layout.dir() == Direction::Vertical && self.layout.is_reversed() {
            debug_assert_eq!(self.min_rect.max.y, self.max_rect.max.y);
            self.min_rect.min.y = self.min_rect.min.y.min(self.min_rect.max.y - height);
        } else {
            debug_assert_eq!(self.min_rect.min.y, self.max_rect.min.y);
            self.min_rect.max.y = self.min_rect.max.y.max(self.min_rect.min.y + height);
        }
        self.max_rect = self.max_rect.union(self.min_rect);
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
        self.min_rect = self.min_rect.union(rect);
        self.max_rect = self.max_rect.union(rect);
    }

    // ------------------------------------------------------------------------
    // Layout related measures:

    /// The available space at the moment, given the current cursor.
    /// This how much more space we can take up without overflowing our parent.
    /// Shrinks as widgets allocate space and the cursor moves.
    /// A small rectangle should be interpreted as "as little as possible".
    /// An infinite rectangle should be interpreted as "as much as you want".
    /// In most layouts the next widget will be put in the top left corner of this `Rect`.
    pub fn available(&self) -> Rect {
        self.layout.available(self.cursor, self.max_rect())
    }

    /// This is like `available()`, but will never be infinite.
    /// Use this for components that want to grow without bounds (but shouldn't).
    /// In most layouts the next widget will be put in the top left corner of this `Rect`.
    pub fn available_finite(&self) -> Rect {
        self.layout.available(self.cursor, self.max_rect_finite())
    }
}

/// # `Id` creation
impl Ui {
    /// Will warn if the returned id is not guaranteed unique.
    /// Use this to generate widget ids for widgets that have persistent state in Memory.
    /// If the `id_source` is not unique within this ui
    /// then an error will be printed at the current cursor position.
    pub fn make_unique_child_id<IdSource>(&self, id_source: IdSource) -> Id
    where
        IdSource: Hash + std::fmt::Debug,
    {
        let id = self.id.with(&id_source);
        // TODO: clip name clash error messages to clip rect
        self.ctx().register_unique_id(id, id_source, self.cursor)
    }

    /// Ideally, all widgets should use this. TODO
    /// Widgets can set an explicit id source (user picked, e.g. some loop index),
    /// and a default id source (e.g. label).
    /// If they fail to be unique, a positional id will be used instead.
    pub fn make_unique_child_id_full(
        &mut self,
        explicit_id_source: Option<Id>,
        default_id_source: Option<&str>,
    ) -> Id {
        let id = if let Some(explicit_id_source) = explicit_id_source {
            self.id.with(&explicit_id_source)
        } else {
            let id = self.id.with(default_id_source);
            if self.ctx().is_unique_id(id) {
                id
            } else {
                self.make_position_id()
            }
        };
        self.ctx()
            .register_unique_id(id, default_id_source.unwrap_or_default(), self.cursor)
    }

    /// Make an Id that is unique to this position.
    /// Can be used for widgets that do NOT persist state in Memory
    /// but you still need to interact with (e.g. buttons, sliders).
    pub fn make_position_id(&self) -> Id {
        self.id.with(self.child_count)
    }

    pub fn make_child_id(&self, id_seed: impl Hash) -> Id {
        self.id.with(id_seed)
    }
}

/// # Interaction
impl Ui {
    pub fn interact(&self, rect: Rect, id: Id, sense: Sense) -> Response {
        self.ctx()
            .interact(self.layer(), self.clip_rect(), rect, Some(id), sense)
    }

    pub fn interact_hover(&self, rect: Rect) -> Response {
        self.ctx()
            .interact(self.layer(), self.clip_rect(), rect, None, Sense::nothing())
    }

    pub fn hovered(&self, rect: Rect) -> bool {
        self.interact_hover(rect).hovered
    }

    pub fn contains_mouse(&self, rect: Rect) -> bool {
        self.ctx()
            .contains_mouse(self.layer(), self.clip_rect(), rect)
    }

    // ------------------------------------------------------------------------
    // Stuff that moves the cursor, i.e. allocates space in this ui!

    /// Advance the cursor (where the next widget is put) by this many points.
    /// The direction is dependent on the layout.
    /// This is useful for creating some extra space between widgets.
    pub fn advance_cursor(&mut self, amount: f32) {
        self.layout.advance_cursor(&mut self.cursor, amount);
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
    /// for `Justified` aligned layouts, like in menus.
    ///
    /// You may get LESS space than you asked for if the current layout won't fit what you asked for.
    pub fn allocate_space(&mut self, desired_size: Vec2) -> Rect {
        let desired_size = self.painter().round_vec_to_pixels(desired_size);
        self.cursor = self.painter().round_pos_to_pixels(self.cursor);

        // For debug rendering
        let too_wide = desired_size.x > self.available().width();
        let too_high = desired_size.x > self.available().height();

        let rect = self.reserve_space_impl(desired_size);

        if self.style().visuals.debug_widget_rects {
            self.painter.rect_stroke(rect, 0.0, (1.0, LIGHT_BLUE));

            let color = color::srgba(200, 0, 0, 255);
            let width = 2.5;

            let paint_line_seg = |a, b| self.painter().line_segment([a, b], (width, color));

            if too_wide {
                paint_line_seg(rect.left_top(), rect.left_bottom());
                paint_line_seg(rect.left_center(), rect.right_center());
                paint_line_seg(rect.right_top(), rect.right_bottom());
            }

            if too_high {
                paint_line_seg(rect.left_top(), rect.right_top());
                paint_line_seg(rect.center_top(), rect.center_bottom());
                paint_line_seg(rect.left_bottom(), rect.right_bottom());
            }
        }

        rect
    }

    /// Reserve this much space and move the cursor.
    /// Returns where to put the widget.
    fn reserve_space_impl(&mut self, child_size: Vec2) -> Rect {
        let available_size = self.available_finite().size();
        let child_rect =
            self.layout
                .allocate_space(&mut self.cursor, &self.style, available_size, child_size);
        self.min_rect = self.min_rect.union(child_rect);
        self.max_rect = self.max_rect.union(child_rect);
        self.child_count += 1;
        child_rect
    }
}

/// # Adding widgets
impl Ui {
    pub fn add(&mut self, widget: impl Widget) -> Response {
        widget.ui(self)
    }

    /// Shortcut for `add(Label::new(text))`
    pub fn label(&mut self, label: impl Into<Label>) -> Response {
        self.add(label.into())
    }

    /// Shortcut for `add(Label::new(text).heading())`
    pub fn heading(&mut self, label: impl Into<Label>) -> Response {
        self.add(label.into().heading())
    }

    /// Shortcut for `add(Label::new(text).monospace())`
    pub fn monospace(&mut self, label: impl Into<Label>) -> Response {
        self.add(label.into().monospace())
    }

    /// Shortcut for `add(Hyperlink::new(url))`
    pub fn hyperlink(&mut self, url: impl Into<String>) -> Response {
        self.add(Hyperlink::new(url))
    }

    pub fn text_edit(&mut self, text: &mut String) -> Response {
        self.add(TextEdit::new(text))
    }

    /// Shortcut for `add(Button::new(text))`
    #[must_use = "You should check if the user clicked this with `if ui.button(...).clicked { ... } "]
    pub fn button(&mut self, text: impl Into<String>) -> Response {
        self.add(Button::new(text))
    }

    /// Show a checkbox.
    pub fn checkbox(&mut self, checked: &mut bool, text: impl Into<String>) -> Response {
        self.add(Checkbox::new(checked, text))
    }

    /// Show a radio button.
    pub fn radio(&mut self, checked: bool, text: impl Into<String>) -> Response {
        self.add(RadioButton::new(checked, text))
    }

    /// Show a radio button. It is selected if `*current_value == radio_value`.
    /// If clicked, `radio_value` is assigned to `*current_value`;
    pub fn radio_value<Value: PartialEq>(
        &mut self,
        current_value: &mut Value,
        radio_value: Value,
        text: impl Into<String>,
    ) -> Response {
        let response = self.radio(*current_value == radio_value, text);
        if response.clicked {
            *current_value = radio_value;
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

    /// Show an image here with the given size
    pub fn image(&mut self, texture_id: TextureId, desired_size: Vec2) -> Response {
        self.add(Image::new(texture_id, desired_size))
    }
}

/// # Colors
impl Ui {
    /// Shows a button with the given color.
    /// If the user clicks the button, a full color picker is shown.
    pub fn color_edit_button_srgba(&mut self, srgba: &mut Srgba) -> Response {
        widgets::color_picker::color_edit_button_srgba(self, srgba)
    }

    /// Shows a button with the given color.
    /// If the user clicks the button, a full color picker is shown.
    pub fn color_edit_button_hsva(&mut self, hsva: &mut Hsva) -> Response {
        widgets::color_picker::color_edit_button_hsva(self, hsva)
    }

    /// Shows a button with the given color.
    /// If the user clicks the button, a full color picker is shown.
    /// The given color is in `sRGBA` space with premultiplied alpha
    pub fn color_edit_button_srgba_premultiplied(&mut self, srgba: &mut [u8; 4]) -> Response {
        let mut color = Srgba(*srgba);
        let response = self.color_edit_button_srgba(&mut color);
        *srgba = color.0;
        response
    }

    /// Shows a button with the given color.
    /// If the user clicks the button, a full color picker is shown.
    /// The given color is in `sRGBA` space without premultiplied alpha.
    /// If unsure, what "premultiplied alpha" is, then this is probably the function you want to use.
    pub fn color_edit_button_srgba_unmultiplied(&mut self, srgba: &mut [u8; 4]) -> Response {
        let mut hsva = Hsva::from_srgba_unmultiplied(*srgba);
        let response = self.color_edit_button_hsva(&mut hsva);
        *srgba = hsva.to_srgba_unmultiplied();
        response
    }

    /// Shows a button with the given color.
    /// If the user clicks the button, a full color picker is shown.
    /// The given color is in linear RGBA space with premultiplied alpha
    pub fn color_edit_button_rgba_premultiplied(&mut self, rgba: &mut [f32; 4]) -> Response {
        let mut hsva = Hsva::from_rgba_premultiplied(*rgba);
        let response = self.color_edit_button_hsva(&mut hsva);
        *rgba = hsva.to_rgba_premultiplied();
        response
    }

    /// Shows a button with the given color.
    /// If the user clicks the button, a full color picker is shown.
    /// The given color is in linear RGBA space without premultiplied alpha.
    /// If unsure, what "premultiplied alpha" is, then this is probably the function you want to use.
    pub fn color_edit_button_rgba_unmultiplied(&mut self, rgba: &mut [f32; 4]) -> Response {
        let mut hsva = Hsva::from_rgba_unmultiplied(*rgba);
        let response = self.color_edit_button_hsva(&mut hsva);
        *rgba = hsva.to_rgba_unmultiplied();
        response
    }
}

/// # Adding Containers / Sub-uis:
impl Ui {
    pub fn collapsing<R>(
        &mut self,
        heading: impl Into<String>,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<R> {
        CollapsingHeader::new(heading).show(self, add_contents)
    }

    /// Create a child ui at the current cursor.
    /// `size` is the desired size.
    /// Actual size may be much smaller if `available_size()` is not enough.
    /// Set `size` to `Vec::infinity()` to get as much space as possible.
    /// Just because you ask for a lot of space does not mean you have to use it!
    /// After `add_contents` is called the contents of `min_size`
    /// will decide how much space will be used in the parent ui.
    pub fn add_custom_contents(&mut self, size: Vec2, add_contents: impl FnOnce(&mut Ui)) -> Rect {
        let size = size.at_most(self.available().size());
        let child_rect = self.layout.rect_from_cursor_size(self.cursor, size);
        let mut child_ui = self.child_ui(child_rect, self.layout);
        add_contents(&mut child_ui);
        self.allocate_space(child_ui.min_size())
    }

    /// Create a child ui. You can use this to temporarily change the Style of a sub-region, for instance.
    pub fn add_custom<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> (R, Rect) {
        let child_rect = self.available();
        let mut child_ui = self.child_ui(child_rect, self.layout);
        let r = add_contents(&mut child_ui);
        let size = child_ui.min_size();
        (r, self.allocate_space(size))
    }

    /// Create a child ui which is indented to the right
    pub fn indent<R>(
        &mut self,
        id_source: impl Hash,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> (R, Rect) {
        assert!(
            self.layout().dir() == Direction::Vertical,
            "You can only indent vertical layouts"
        );
        let indent = vec2(self.style().spacing.indent, 0.0);
        let child_rect = Rect::from_min_max(self.cursor + indent, self.max_rect.right_bottom()); // TODO: wrong for reversed layouts
        let mut child_ui = Ui {
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

        (ret, self.allocate_space(indent + size))
    }

    pub fn left_column(&mut self, width: f32) -> Ui {
        self.column(Align::Min, width)
    }

    pub fn centered_column(&mut self, width: f32) -> Ui {
        self.column(Align::Center, width)
    }

    pub fn right_column(&mut self, width: f32) -> Ui {
        self.column(Align::Max, width)
    }

    /// A column ui with a given width.
    pub fn column(&mut self, column_position: Align, width: f32) -> Ui {
        let x = match column_position {
            Align::Min => 0.0,
            Align::Center => self.available().width() / 2.0 - width / 2.0,
            Align::Max => self.available().width() - width,
        };
        self.child_ui(
            Rect::from_min_size(
                self.cursor + vec2(x, 0.0),
                vec2(width, self.available().height()),
            ),
            self.layout,
        )
    }

    /// Start a ui with horizontal layout.
    /// After you have called this, the registers the contents as any other widget.
    ///
    /// Elements will be centered on the Y axis, i.e.
    /// adjusted up and down to lie in the center of the horizontal layout.
    /// The initial height is `style.spacing.interact_size.y`.
    /// Centering is almost always what you want if you are
    /// planning to to mix widgets or just different types of text.
    pub fn horizontal<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> (R, Rect) {
        let initial_size = vec2(
            self.available().width(),
            self.style().spacing.interact_size.y, // Assume there will be something interactive on the horizontal layout
        );

        let right_to_left =
            (self.layout.dir(), self.layout.align()) == (Direction::Vertical, Some(Align::Max));

        self.inner_layout(
            Layout::horizontal(Align::Center).with_reversed(right_to_left),
            initial_size,
            add_contents,
        )
    }

    /// Start a ui with vertical layout.
    /// Widgets will be left-justified.
    pub fn vertical<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> (R, Rect) {
        self.with_layout(Layout::vertical(Align::Min), add_contents)
    }

    pub fn inner_layout<R>(
        &mut self,
        layout: Layout,
        initial_size: Vec2,
        add_contents: impl FnOnce(&mut Self) -> R,
    ) -> (R, Rect) {
        let child_rect = self.layout.rect_from_cursor_size(self.cursor, initial_size);
        let mut child_ui = self.child_ui(child_rect, layout);
        let ret = add_contents(&mut child_ui);
        let size = child_ui.min_size();
        let rect = self.allocate_space(size);
        (ret, rect)
    }

    pub fn with_layout<R>(
        &mut self,
        layout: Layout,
        add_contents: impl FnOnce(&mut Self) -> R,
    ) -> (R, Rect) {
        let mut child_ui = self.child_ui(self.available(), layout);
        let ret = add_contents(&mut child_ui);
        let size = child_ui.min_size();
        let rect = self.allocate_space(size);
        (ret, rect)
    }

    /// Temporarily split split an Ui into several columns.
    ///
    /// ``` ignore
    /// ui.columns(2, |columns| {
    ///     columns[0].add(egui::widgets::label!("First column"));
    ///     columns[1].add(egui::widgets::label!("Second column"));
    /// });
    /// ```
    pub fn columns<F, R>(&mut self, num_columns: usize, add_contents: F) -> R
    where
        F: FnOnce(&mut [Self]) -> R,
    {
        // TODO: ensure there is space
        let spacing = self.style().spacing.item_spacing.x;
        let total_spacing = spacing * (num_columns as f32 - 1.0);
        let column_width = (self.available().width() - total_spacing) / (num_columns as f32);

        let mut columns: Vec<Self> = (0..num_columns)
            .map(|col_idx| {
                let pos = self.cursor + vec2((col_idx as f32) * (column_width + spacing), 0.0);
                let child_rect = Rect::from_min_max(
                    pos,
                    pos2(pos.x + column_width, self.max_rect.right_bottom().y),
                );

                Self {
                    id: self.make_child_id(&("column", col_idx)),
                    ..self.child_ui(child_rect, self.layout)
                }
            })
            .collect();

        let result = add_contents(&mut columns[..]);

        let mut sum_width = total_spacing;
        for column in &columns {
            sum_width += column.min_rect.width();
        }

        let mut max_height = 0.0;
        for ui in columns {
            let size = ui.min_size();
            max_height = size.y.max(max_height);
        }

        let size = vec2(self.available().width().max(sum_width), max_height);
        self.allocate_space(size);
        result
    }
}

// ----------------------------------------------------------------------------

/// ## Debug stuff
impl Ui {
    /// Shows where the next widget is going to be placed
    pub fn debug_paint_cursor(&self) {
        self.layout.debug_paint_cursor(self.cursor, &self.painter);
    }
}
