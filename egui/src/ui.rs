use std::{hash::Hash, sync::Arc};

use crate::{color::*, containers::*, layout::*, paint::*, widgets::*, *};

/// Represents a region of the screen
/// with a type of layout (horizontal or vertical).
pub struct Ui {
    /// How we access input, output and memory
    ctx: Arc<Context>,

    /// ID of this ui.
    /// Generated based on id of parent ui together with
    /// another source of child identity (e.g. window title).
    /// Acts like a namespace for child uis.
    /// Hopefully unique.
    id: Id,

    /// Where to put the graphics output of this Ui
    layer: Layer,

    /// Everything painted in this ui will be clipped against this.
    /// This means nothing outside of this rectangle will be visible on screen.
    clip_rect: Rect,

    /// The `rect` represents where in screen-space the ui is
    /// and its max size (original available_space).
    /// Note that the size may be infinite in one or both dimensions.
    /// The widgets will TRY to fit within the rect,
    /// but may overflow (which you will see in child_bounds).
    /// Some widgets (like separator lines) will try to fill the full desired width of the ui.
    /// If the desired size is zero, it is a signal that child widgets should be as small as possible.
    /// If the desired size is initie, it is a signal that child widgets should take up as much room as they want.
    desired_rect: Rect, // TODO: rename as max_rect ?

    /// Bounding box of all children.
    /// This is used to see how large a ui actually
    /// needs to be after all children has been added.
    /// You can think of this as the minimum size.
    child_bounds: Rect, // TODO: rename as min_rect ?

    /// Overide default style in this ui
    style: Style,

    layout: Layout,

    /// Where the next widget will be put.
    /// Progresses along self.dir.
    /// Initially set to rect.min
    /// If something has already been added, this will point ot style.item_spacing beyond the latest child.
    /// The cursor can thus be style.item_spacing pixels outside of the child_bounds.
    cursor: Pos2, // TODO: move into Layout?
}

impl Ui {
    // ------------------------------------------------------------------------
    // Creation:

    pub fn new(ctx: Arc<Context>, layer: Layer, id: Id, rect: Rect) -> Self {
        let style = ctx.style();
        Ui {
            ctx,
            id,
            layer,
            clip_rect: rect.expand(style.clip_rect_margin),
            desired_rect: rect,
            child_bounds: Rect::from_min_size(rect.min, Vec2::zero()), // TODO: Rect::nothing() ?
            style,
            layout: Default::default(),
            cursor: rect.min,
        }
    }

    pub fn child_ui(&self, child_rect: Rect) -> Self {
        // let clip_rect = self
        //     .clip_rect
        //     .intersect(&child_rect.expand(self.style().clip_rect_margin));
        let clip_rect = self.clip_rect(); // Keep it unless the child explciitly desires differently
        Ui {
            ctx: self.ctx.clone(),
            id: self.id,
            layer: self.layer,
            clip_rect,
            desired_rect: child_rect,
            child_bounds: Rect::from_min_size(child_rect.min, Vec2::zero()), // TODO: Rect::nothing() ?
            style: self.style.clone(),
            layout: self.layout,
            cursor: child_rect.min,
        }
    }

    // -------------------------------------------------

    pub fn round_to_pixel(&self, point: f32) -> f32 {
        self.ctx.round_to_pixel(point)
    }

    pub fn round_vec_to_pixels(&self, vec: Vec2) -> Vec2 {
        self.ctx.round_vec_to_pixels(vec)
    }

    pub fn round_pos_to_pixels(&self, pos: Pos2) -> Pos2 {
        self.ctx.round_pos_to_pixels(pos)
    }

    pub fn id(&self) -> Id {
        self.id
    }

    /// Options for this ui, and any child uis we may spawn.
    pub fn style(&self) -> &Style {
        &self.style
    }

    pub fn set_style(&mut self, style: Style) {
        self.style = style
    }

    pub fn ctx(&self) -> &Arc<Context> {
        &self.ctx
    }

    pub fn input(&self) -> &InputState {
        self.ctx.input()
    }

    pub fn memory(&self) -> parking_lot::MutexGuard<'_, Memory> {
        self.ctx.memory()
    }

    pub fn output(&self) -> parking_lot::MutexGuard<'_, Output> {
        self.ctx.output()
    }

    pub fn fonts(&self) -> &Fonts {
        self.ctx.fonts()
    }

    /// Screen-space rectangle for clipping what we paint in this ui.
    /// This is used, for instance, to avoid painting outside a window that is smaller
    /// than its contents.
    pub fn clip_rect(&self) -> Rect {
        self.clip_rect
    }

    pub fn set_clip_rect(&mut self, clip_rect: Rect) {
        self.clip_rect = clip_rect;
    }

    // ------------------------------------------------------------------------

    /// Screen-space position of this Ui.
    /// This may have moved from its original if a child overflowed to the left or up (rare).
    pub fn top_left(&self) -> Pos2 {
        // If a child doesn't fit in desired_rect, we have effectively expanded:
        self.desired_rect.min.min(self.child_bounds.min)
    }

    /// Screen-space position of the current bottom right corner of this Ui.
    /// This may move when we add children that overflow our desired rectangle bounds.
    /// This position may be at inifnity if the desired rect is initinite,
    /// which mappens when a parent widget says "be as big as you want to be".
    pub fn bottom_right(&self) -> Pos2 {
        // If a child doesn't fit in desired_rect, we have effectively expanded:
        self.desired_rect.max.max(self.child_bounds.max)
    }

    /// Position and current size of the ui.
    /// The size is the maximum of the origional (minimum/desired) size and
    /// the size of the containted children.
    pub fn rect(&self) -> Rect {
        Rect::from_min_max(self.top_left(), self.bottom_right())
    }

    /// This is like `rect()`, but will never be infinite.
    /// If the desired rect is infinite ("be as big as you want")
    /// this will be bounded by child bounds.
    pub fn rect_finite(&self) -> Rect {
        let mut bottom_right = self.child_bounds.max;
        if self.desired_rect.max.x.is_finite() {
            bottom_right.x = bottom_right.x.max(self.desired_rect.max.x);
        }
        if self.desired_rect.max.y.is_finite() {
            bottom_right.y = bottom_right.y.max(self.desired_rect.max.y);
        }

        Rect::from_min_max(self.top_left(), bottom_right)
    }

    /// Set the width of the ui.
    /// You won't be able to shrink it beyond its current child bounds.
    pub fn set_desired_width(&mut self, width: f32) {
        let min_width = self.child_bounds.max.x - self.top_left().x;
        let width = width.max(min_width);
        self.desired_rect.max.x = self.top_left().x + width;
    }

    /// Set the height of the ui.
    /// You won't be able to shrink it beyond its current child bounds.
    pub fn set_desired_height(&mut self, height: f32) {
        let min_height = self.child_bounds.max.y - self.top_left().y;
        let height = height.max(min_height);
        self.desired_rect.max.y = self.top_left().y + height;
    }

    /// Size of content
    pub fn bounding_size(&self) -> Vec2 {
        self.child_bounds.size()
    }

    /// Expand the bounding rect of this ui to include a child at the given rect.
    pub fn expand_to_include_child(&mut self, rect: Rect) {
        self.child_bounds.extend_with(rect.min);
        self.child_bounds.extend_with(rect.max);
    }

    pub fn expand_to_size(&mut self, size: Vec2) {
        self.child_bounds.extend_with(self.top_left() + size);
    }

    /// Bounding box of all contained children
    pub fn child_bounds(&self) -> Rect {
        self.child_bounds
    }

    pub fn force_set_child_bounds(&mut self, child_bounds: Rect) {
        self.child_bounds = child_bounds;
    }

    // ------------------------------------------------------------------------
    // Layout related measures:

    /// The available space at the moment, given the current cursor.
    /// This how much more space we can take up without overflowing our parent.
    /// Shrinks as widgets allocate space and the cursor moves.
    /// A small rectangle should be intepreted as "as little as possible".
    /// An infinite rectangle should be interpred as "as much as you want".
    /// In most layouts the next widget will be put in the top left corner of this `Rect`.
    pub fn available(&self) -> Rect {
        self.layout.available(self.cursor, self.rect())
    }

    /// This is like `available()`, but will never be infinite.
    /// Use this for components that want to grow without bounds (but shouldn't).
    /// In most layouts the next widget will be put in the top left corner of this `Rect`.
    pub fn available_finite(&self) -> Rect {
        self.layout.available(self.cursor, self.rect_finite())
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    // TODO: remove
    pub fn set_layout(&mut self, layout: Layout) {
        self.layout = layout;

        // TODO: remove this HACK:
        if layout.is_reversed() {
            self.cursor = self.rect_finite().max;
        }
    }

    // ------------------------------------------------------------------------

    pub fn contains_mouse(&self, rect: Rect) -> bool {
        self.ctx.contains_mouse(self.layer, self.clip_rect, rect)
    }

    pub fn has_kb_focus(&self, id: Id) -> bool {
        self.memory().kb_focus_id == Some(id)
    }

    pub fn request_kb_focus(&self, id: Id) {
        self.memory().kb_focus_id = Some(id);
    }

    // ------------------------------------------------------------------------

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
        self.ctx.register_unique_id(id, id_source, self.cursor)
    }

    /// Make an Id that is unique to this positon.
    /// Can be used for widgets that do NOT persist state in Memory
    /// but you still need to interact with (e.g. buttons, sliders).
    pub fn make_position_id(&self) -> Id {
        self.id.with(&Id::from_pos(self.cursor))
    }

    pub fn make_child_id(&self, id_seed: impl Hash) -> Id {
        self.id.with(id_seed)
    }

    // ------------------------------------------------------------------------
    // Interaction

    pub fn interact(&self, rect: Rect, id: Id, sense: Sense) -> InteractInfo {
        self.ctx
            .interact(self.layer, self.clip_rect, rect, Some(id), sense)
    }

    pub fn interact_hover(&self, rect: Rect) -> InteractInfo {
        self.ctx
            .interact(self.layer, self.clip_rect, rect, None, Sense::nothing())
    }

    pub fn hovered(&self, rect: Rect) -> bool {
        self.interact_hover(rect).hovered
    }

    #[must_use]
    pub fn response(&mut self, interact: InteractInfo) -> GuiResponse {
        // TODO: unify GuiResponse and InteractInfo. They are the same thing!
        GuiResponse {
            hovered: interact.hovered,
            clicked: interact.clicked,
            double_clicked: interact.double_clicked,
            active: interact.active,
            rect: interact.rect,
            ctx: self.ctx.clone(),
        }
    }

    // ------------------------------------------------------------------------
    // Stuff that moves the cursor, i.e. allocates space in this ui!

    /// Reserve this much space and move the cursor.
    /// Returns where to put the widget.
    ///
    /// # How sizes are negotiated
    /// Each widget should have a *minimum desired size* and a *desired size*.
    /// When asking for space, ask AT LEAST for you minimum, and don't ask for more than you need.
    /// If you want to fill the space, ask about `available().size()` and use that.
    ///
    /// You may get MORE space than you asked for, for instance
    /// for `Justified` aligned layouts, like in menus.
    ///
    /// You may get LESS space than you asked for if the current layout won't fit what you asked for.
    pub fn allocate_space(&mut self, child_size: Vec2) -> Rect {
        let child_size = self.round_vec_to_pixels(child_size);
        self.cursor = self.round_pos_to_pixels(self.cursor);

        // For debug rendering
        let too_wide = child_size.x > self.available().width();
        let too_high = child_size.x > self.available().height();

        let rect = self.reserve_space_impl(child_size);

        if self.style().debug_widget_rects {
            self.add_paint_cmd(PaintCmd::Rect {
                rect,
                corner_radius: 0.0,
                outline: Some(LineStyle::new(1.0, LIGHT_BLUE)),
                fill: None,
            });

            let color = color::srgba(200, 0, 0, 255);
            let width = 2.5;

            let mut paint_line_seg =
                |a, b| self.add_paint_cmd(PaintCmd::line_segment([a, b], color, width));

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
        self.child_bounds = self.child_bounds.union(child_rect);
        child_rect
    }

    // ------------------------------------------------
    // Painting related stuff

    /// It is up to the caller to make sure there is room for this.
    /// Can be used for free painting.
    /// NOTE: all coordinates are screen coordinates!
    pub fn add_paint_cmd(&mut self, paint_cmd: PaintCmd) {
        self.ctx
            .graphics()
            .layer(self.layer)
            .push((self.clip_rect(), paint_cmd))
    }

    pub fn add_paint_cmds(&mut self, mut cmds: Vec<PaintCmd>) {
        let clip_rect = self.clip_rect();
        self.ctx
            .graphics()
            .layer(self.layer)
            .extend(cmds.drain(..).map(|cmd| (clip_rect, cmd)));
    }

    /// Insert a paint cmd before existing ones
    pub fn insert_paint_cmd(&mut self, pos: usize, paint_cmd: PaintCmd) {
        self.ctx
            .graphics()
            .layer(self.layer)
            .insert(pos, (self.clip_rect(), paint_cmd));
    }

    pub fn paint_list_len(&self) -> usize {
        self.ctx.graphics().layer(self.layer).len()
    }

    /// Paint some debug text at current cursor
    pub fn debug_text(&self, text: impl Into<String>) {
        self.debug_text_at(self.cursor, text);
    }

    pub fn debug_text_at(&self, pos: Pos2, text: impl Into<String>) {
        self.ctx.debug_text(pos, text);
    }

    pub fn debug_rect(&mut self, rect: Rect, text: impl Into<String>) {
        self.add_paint_cmd(PaintCmd::Rect {
            corner_radius: 0.0,
            fill: None,
            outline: Some(LineStyle::new(1.0, color::RED)),
            rect,
        });
        let align = (Align::Min, Align::Min);
        let text_style = TextStyle::Monospace;
        self.floating_text(rect.min, text.into(), text_style, align, Some(color::RED));
    }

    /// Show some text anywhere in the ui.
    /// To center the text at the given position, use `align: (Center, Center)`.
    /// If you want to draw text floating on top of everything,
    /// consider using `Context.floating_text` instead.
    pub fn floating_text(
        &mut self,
        pos: Pos2,
        text: impl Into<String>,
        text_style: TextStyle,
        align: (Align, Align),
        text_color: Option<Color>,
    ) -> Rect {
        let font = &self.fonts()[text_style];
        let galley = font.layout_multiline(text.into(), f32::INFINITY);
        let rect = align_rect(Rect::from_min_size(pos, galley.size), align);
        self.add_galley(rect.min, galley, text_style, text_color);
        rect
    }

    /// Already layed out text.
    pub fn add_galley(
        &mut self,
        pos: Pos2,
        galley: font::Galley,
        text_style: TextStyle,
        color: Option<Color>,
    ) {
        let color = color.unwrap_or_else(|| self.style().text_color);
        self.add_paint_cmd(PaintCmd::Text {
            pos,
            galley,
            text_style,
            color,
        });
    }

    // ------------------------------------------------------------------------
    // Addding Widgets

    pub fn add(&mut self, widget: impl Widget) -> GuiResponse {
        let interact = widget.ui(self);
        self.response(interact)
    }

    // Convenience functions:

    pub fn label(&mut self, label: impl Into<Label>) -> GuiResponse {
        self.add(label.into())
    }

    pub fn hyperlink(&mut self, url: impl Into<String>) -> GuiResponse {
        self.add(Hyperlink::new(url))
    }

    pub fn button(&mut self, text: impl Into<String>) -> GuiResponse {
        self.add(Button::new(text))
    }

    // TODO: argument order?
    pub fn checkbox(&mut self, text: impl Into<String>, checked: &mut bool) -> GuiResponse {
        self.add(Checkbox::new(checked, text))
    }

    // TODO: argument order?
    pub fn radio(&mut self, text: impl Into<String>, checked: bool) -> GuiResponse {
        self.add(RadioButton::new(checked, text))
    }

    pub fn separator(&mut self) -> GuiResponse {
        self.add(Separator::new())
    }

    // ------------------------------------------------------------------------
    // Addding Containers / Sub-uis:

    pub fn collapsing<R>(
        &mut self,
        text: impl Into<String>,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<R> {
        CollapsingHeader::new(text).show(self, add_contents)
    }

    /// Create a child ui at the current cursor.
    /// `size` is the desired size.
    /// Actual size may be much smaller if `avilable_size()` is not enough.
    /// Set `size` to `Vec::infinity()` to get as much space as possible.
    /// Just because you ask for a lot of space does not mean you have to use it!
    /// After `add_contents` is called the contents of `bounding_size`
    /// will decide how much space will be used in the parent ui.
    pub fn add_custom_contents(&mut self, size: Vec2, add_contents: impl FnOnce(&mut Ui)) -> Rect {
        let size = size.min(self.available().size());
        let child_rect = Rect::from_min_size(self.cursor, size);
        let mut child_ui = self.child_ui(child_rect);
        add_contents(&mut child_ui);
        self.allocate_space(child_ui.bounding_size())
    }

    /// Create a child ui
    pub fn add_custom<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> (R, Rect) {
        let child_rect = self.available();
        let mut child_ui = self.child_ui(child_rect);
        let r = add_contents(&mut child_ui);
        let size = child_ui.bounding_size();
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
        let indent = vec2(self.style.indent, 0.0);
        let child_rect = Rect::from_min_max(self.cursor + indent, self.bottom_right());
        let mut child_ui = Ui {
            id: self.id.with(id_source),
            ..self.child_ui(child_rect)
        };
        let ret = add_contents(&mut child_ui);
        let size = child_ui.bounding_size();

        // draw a grey line on the left to mark the indented section
        let line_start = child_rect.min - indent * 0.5;
        let line_start = self.round_pos_to_pixels(line_start);
        let line_end = pos2(line_start.x, line_start.y + size.y - 2.0);
        self.add_paint_cmd(PaintCmd::line_segment(
            [line_start, line_end],
            gray(150, 255),
            self.style.line_width,
        ));

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
        self.child_ui(Rect::from_min_size(
            self.cursor + vec2(x, 0.0),
            vec2(width, self.available().height()),
        ))
    }

    /// Start a ui with horizontal layout
    pub fn horizontal<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> (R, Rect) {
        self.inner_layout(Layout::horizontal(Align::Min), add_contents)
    }

    /// Start a ui with vertical layout
    pub fn vertical<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> (R, Rect) {
        self.inner_layout(Layout::vertical(Align::Min), add_contents)
    }

    pub fn inner_layout<R>(
        &mut self,
        layout: Layout,
        add_contents: impl FnOnce(&mut Self) -> R,
    ) -> (R, Rect) {
        let child_rect = Rect::from_min_max(self.cursor, self.bottom_right());
        let mut child_ui = Self {
            ..self.child_ui(child_rect)
        };
        child_ui.set_layout(layout); // HACK: need a separate call right now
        let ret = add_contents(&mut child_ui);
        let size = child_ui.bounding_size();
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
        let spacing = self.style.item_spacing.x;
        let total_spacing = spacing * (num_columns as f32 - 1.0);
        let column_width = (self.available().width() - total_spacing) / (num_columns as f32);

        let mut columns: Vec<Self> = (0..num_columns)
            .map(|col_idx| {
                let pos = self.cursor + vec2((col_idx as f32) * (column_width + spacing), 0.0);
                let child_rect =
                    Rect::from_min_max(pos, pos2(pos.x + column_width, self.bottom_right().y));

                Self {
                    id: self.make_child_id(&("column", col_idx)),
                    ..self.child_ui(child_rect)
                }
            })
            .collect();

        let result = add_contents(&mut columns[..]);

        let mut sum_width = total_spacing;
        for column in &columns {
            sum_width += column.child_bounds.width();
        }

        let mut max_height = 0.0;
        for ui in columns {
            let size = ui.bounding_size();
            max_height = size.y.max(max_height);
        }

        let size = vec2(self.available().width().max(sum_width), max_height);
        self.allocate_space(size);
        result
    }

    // ------------------------------------------------
}
