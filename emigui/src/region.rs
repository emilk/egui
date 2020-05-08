use std::{hash::Hash, sync::Arc};

use crate::{color::*, containers::*, font::TextFragment, layout::*, widgets::*, *};

/// Represents a region of the screen
/// with a type of layout (horizontal or vertical).
/// TODO: make Region a trait so we can have type-safe `HorizontalRegion` etc?
pub struct Region {
    /// How we access input, output and memory
    ctx: Arc<Context>,

    /// ID of this region.
    /// Generated based on id of parent region together with
    /// another source of child identity (e.g. window title).
    /// Acts like a namespace for child regions.
    /// Hopefully unique.
    id: Id,

    /// Where to put the graphics output of this Region
    layer: Layer,

    /// Everything painted in this region will be clipped against this.
    /// This means nothing outside of this rectangle will be visible on screen.
    clip_rect: Rect,

    /// The `rect` represents where in space the region is
    /// and its max size (original available_space).
    /// Note that the size may be infinite in one or both dimensions.
    /// The widgets will TRY to fit within the rect,
    /// but may overflow (which you will see in child_bounds).
    /// Some widgets (like separator lines) will try to fill the full desired width of the region.
    desired_rect: Rect, // TODO: rename as max_rect ?

    /// Bounding box of all children.
    /// This is used to see how large a region actually
    /// needs to be after all children has been added.
    /// You can think of this as the minimum size.
    child_bounds: Rect, // TODO: rename as min_rect ?

    /// Overide default style in this region
    style: Style,

    // Layout stuff follows. TODO: move to own type and abstract.
    /// Doesn't change.
    dir: Direction,

    align: Align,

    /// Where the next widget will be put.
    /// Progresses along self.dir.
    /// Initially set to rect.min
    /// If something has already been added, this will point ot style.item_spacing beyond the latest child.
    /// The cursor can thus be style.item_spacing pixels outside of the child_bounds.
    cursor: Pos2,
}

impl Region {
    // ------------------------------------------------------------------------
    // Creation:

    pub fn new(ctx: Arc<Context>, layer: Layer, id: Id, rect: Rect) -> Self {
        let style = ctx.style();
        Region {
            ctx,
            id,
            layer,
            clip_rect: rect.expand(style.clip_rect_margin),
            desired_rect: rect,
            child_bounds: Rect::from_min_size(rect.min, Vec2::zero()), // TODO: Rect::nothing() ?
            style,
            cursor: rect.min,
            dir: Direction::Vertical,
            align: Align::Min,
        }
    }

    pub fn child_region(&self, child_rect: Rect) -> Self {
        // let clip_rect = self
        //     .clip_rect
        //     .intersect(&child_rect.expand(self.style().clip_rect_margin));
        let clip_rect = self.clip_rect(); // Keep it unless the child explciitly desires differently
        Region {
            ctx: self.ctx.clone(),
            layer: self.layer,
            style: self.style,
            id: self.id,
            clip_rect,
            desired_rect: child_rect,
            cursor: child_rect.min,
            child_bounds: Rect::from_min_size(child_rect.min, Vec2::zero()), // TODO: Rect::nothing() ?
            dir: self.dir,
            align: self.align,
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

    /// Options for this region, and any child regions we may spawn.
    pub fn style(&self) -> &Style {
        &self.style
    }

    pub fn ctx(&self) -> &Arc<Context> {
        &self.ctx
    }

    pub fn input(&self) -> &GuiInput {
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

    /// Screen-space rectangle for clipping what we paint in this region.
    /// This is used, for instance, to avoid painting outside a window that is smaller
    /// than its contents.
    pub fn clip_rect(&self) -> Rect {
        self.clip_rect
    }

    pub fn set_clip_rect(&mut self, clip_rect: Rect) {
        self.clip_rect = clip_rect;
    }

    // ------------------------------------------------------------------------

    /// Screen-space position of this Region.
    /// This may have moved from its original if a child overflowed to the left or up (rare).
    pub fn top_left(&self) -> Pos2 {
        // If a child doesn't fit in desired_rect, we have effectively expanded:
        self.desired_rect.min.min(self.child_bounds.min)
    }

    /// Screen-space position of the current bottom right corner of this Region.
    /// This may move when we add children that overflow our desired rectangle bounds.
    pub fn bottom_right(&self) -> Pos2 {
        // If a child doesn't fit in desired_rect, we have effectively expanded:
        self.desired_rect.max.max(self.child_bounds.max)
    }

    /// Position and current size of the region.
    /// The size is the maximum of the origional (minimum/desired) size and
    /// the size of the containted children.
    pub fn rect(&self) -> Rect {
        Rect::from_min_max(self.top_left(), self.bottom_right())
    }

    /// Set the width of the region.
    /// You won't be able to shrink it beyond its current child bounds.
    pub fn set_desired_width(&mut self, width: f32) {
        let min_width = self.child_bounds.max.x - self.top_left().x;
        let width = width.max(min_width);
        self.desired_rect.max.x = self.top_left().x + width;
    }

    /// Set the height of the region.
    /// You won't be able to shrink it beyond its current child bounds.
    pub fn set_desired_height(&mut self, height: f32) {
        let min_height = self.child_bounds.max.y - self.top_left().y;
        let height = height.max(min_height);
        self.desired_rect.max.y = self.top_left().y + height;
    }

    /// Size of content
    pub fn bounding_size(&self) -> Vec2 {
        self.child_bounds.max - self.top_left()
    }

    /// Expand the bounding rect of this region to include a child at the given rect.
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

    pub fn available_width(&self) -> f32 {
        self.available_space().x
    }

    pub fn available_height(&self) -> f32 {
        self.available_space().y
    }

    /// This how much more space we can take up without overflowing our parent.
    /// Shrinks as cursor increments.
    pub fn available_space(&self) -> Vec2 {
        self.bottom_right() - self.cursor
    }

    pub fn direction(&self) -> Direction {
        self.dir
    }

    pub fn cursor(&self) -> Pos2 {
        self.cursor
    }

    pub fn set_align(&mut self, align: Align) {
        self.align = align;
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
    /// If the `id_source` is not unique within this region
    /// then an error will be printed at the current cursor position.
    pub fn make_unique_id<IdSource>(&self, id_source: &IdSource) -> Id
    where
        IdSource: Hash + std::fmt::Debug,
    {
        let id = self.id.with(id_source);
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

    /// Check for clicks on this entire region (rect())
    pub fn interact_whole(&self) -> InteractInfo {
        self.interact_rect(self.rect(), self.id)
    }

    pub fn interact_rect(&self, rect: Rect, id: Id) -> InteractInfo {
        self.ctx
            .interact(self.layer, self.clip_rect, rect, Some(id))
    }

    pub fn response(&mut self, interact: InteractInfo) -> GuiResponse {
        // TODO: unify GuiResponse and InteractInfo. They are the same thing!
        GuiResponse {
            hovered: interact.hovered,
            clicked: interact.clicked,
            active: interact.active,
            rect: interact.rect,
            ctx: self.ctx.clone(),
        }
    }

    // ------------------------------------------------------------------------
    // Stuff that moves the cursor, i.e. allocates space in this region!

    /// Reserve this much space and move the cursor.
    /// Returns where to put the widget.
    /// # How sizes are negotiated
    /// Each widget should have a *minimum desired size* and a *desired size*.
    /// When asking for space, ask AT LEAST for you minimum, and don't ask for more than you need.
    /// If you want to fill the space, ask about `available_space()` and use that.
    /// NOTE: we always get the size we ask for (at the moment).
    pub fn reserve_space(&mut self, child_size: Vec2, interaction_id: Option<Id>) -> InteractInfo {
        let child_size = self.round_vec_to_pixels(child_size);
        self.cursor = self.round_pos_to_pixels(self.cursor);

        // For debug rendering
        let too_wide = child_size.x > self.available_width();
        let too_high = child_size.x > self.available_height();

        let child_pos = self.reserve_space_impl(child_size);
        let rect = Rect::from_min_size(child_pos, child_size);

        if self.style().debug_regions {
            self.add_paint_cmd(PaintCmd::Rect {
                rect,
                corner_radius: 0.0,
                outline: Some(Outline::new(1.0, LIGHT_BLUE)),
                fill_color: None,
            });

            let color = color::srgba(200, 0, 0, 255);
            let width = 2.5;

            if too_wide {
                self.add_paint_cmd(PaintCmd::line_segment(
                    (rect.left_top(), rect.left_bottom()),
                    color,
                    width,
                ));
                self.add_paint_cmd(PaintCmd::line_segment(
                    (rect.left_center(), rect.right_center()),
                    color,
                    width,
                ));
                self.add_paint_cmd(PaintCmd::line_segment(
                    (rect.right_top(), rect.right_bottom()),
                    color,
                    width,
                ));
            }

            if too_high {
                self.add_paint_cmd(PaintCmd::line_segment(
                    (rect.left_top(), rect.right_top()),
                    color,
                    width,
                ));
                self.add_paint_cmd(PaintCmd::line_segment(
                    (rect.center_top(), rect.center_bottom()),
                    color,
                    width,
                ));
                self.add_paint_cmd(PaintCmd::line_segment(
                    (rect.left_bottom(), rect.right_bottom()),
                    color,
                    width,
                ));
            }
        }

        self.ctx
            .interact(self.layer, self.clip_rect, rect, interaction_id)
    }

    /// Reserve this much space and move the cursor.
    /// Returns where to put the widget.
    fn reserve_space_impl(&mut self, child_size: Vec2) -> Pos2 {
        let mut child_pos = self.cursor;
        if self.dir == Direction::Horizontal {
            child_pos.y += match self.align {
                Align::Min => 0.0,
                Align::Center => 0.5 * (self.available_height() - child_size.y),
                Align::Max => self.available_height() - child_size.y,
            };
            self.child_bounds.extend_with(self.cursor + child_size);
            self.cursor.x += child_size.x;
            self.cursor.x += self.style.item_spacing.x; // Where to put next thing, if there is a next thing
        } else {
            child_pos.x += match self.align {
                Align::Min => 0.0,
                Align::Center => 0.5 * (self.available_width() - child_size.x),
                Align::Max => self.available_width() - child_size.x,
            };
            self.child_bounds.extend_with(self.cursor + child_size);
            self.cursor.y += child_size.y;
            self.cursor.y += self.style.item_spacing.y; // Where to put next thing, if there is a next thing
        }

        child_pos
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
    pub fn debug_text(&self, text: &str) {
        self.debug_text_at(self.cursor, text);
    }

    // TODO: AsRef<str>
    pub fn debug_text_at(&self, pos: Pos2, text: &str) {
        self.ctx.debug_text(pos, text);
    }

    /// Show some text anywhere in the region.
    /// To center the text at the given position, use `align: (Center, Center)`.
    /// If you want to draw text floating on top of everything,
    /// consider using `Context.floating_text` instead.
    pub fn floating_text(
        &mut self,
        pos: Pos2,
        text: &str,
        text_style: TextStyle,
        align: (Align, Align),
        text_color: Option<Color>,
    ) -> Vec2 {
        let font = &self.fonts()[text_style];
        let (text, size) = font.layout_multiline(text, f32::INFINITY);
        let rect = align_rect(Rect::from_min_size(pos, size), align);
        self.add_text(rect.min, text_style, text, text_color);
        size
    }

    /// Already layed out text.
    pub fn add_text(
        &mut self,
        pos: Pos2,
        text_style: TextStyle,
        text: Vec<TextFragment>,
        color: Option<Color>,
    ) {
        let color = color.unwrap_or_else(|| self.style().text_color());
        for fragment in text {
            self.add_paint_cmd(PaintCmd::Text {
                color,
                pos: pos + vec2(0.0, fragment.y_offset),
                text: fragment.text,
                text_style,
                x_offsets: fragment.x_offsets,
            });
        }
    }

    // ------------------------------------------------------------------------
    // Addding Widgets

    pub fn add(&mut self, widget: impl Widget) -> GuiResponse {
        widget.ui(self)
    }

    // Convenience functions:

    pub fn add_label(&mut self, text: impl Into<String>) -> GuiResponse {
        self.add(Label::new(text))
    }

    pub fn add_hyperlink(&mut self, url: impl Into<String>) -> GuiResponse {
        self.add(Hyperlink::new(url))
    }

    // ------------------------------------------------------------------------
    // Addding Containers / Sub-Regions:

    pub fn collapsing(
        &mut self,
        text: impl Into<String>,
        add_contents: impl FnOnce(&mut Region),
    ) -> GuiResponse {
        CollapsingHeader::new(text).show(self, add_contents)
    }

    /// Create a child region at the current cursor.
    /// `size` is the desired size.
    /// Actual size may be much smaller if `avilable_size()` is not enough.
    /// Set `size` to `Vec::infinity()` to get as much space as possible.
    /// Just because you ask for a lot of space does not mean you have to use it!
    /// After `add_contents` is called the contents of `bounding_size`
    /// will decide how much space will be used in the parent region.
    pub fn add_custom_contents(&mut self, size: Vec2, add_contents: impl FnOnce(&mut Region)) {
        let size = size.min(self.available_space());
        let child_rect = Rect::from_min_size(self.cursor, size);
        let mut child_region = Region {
            ..self.child_region(child_rect)
        };
        add_contents(&mut child_region);
        self.reserve_space(child_region.bounding_size(), None);
    }

    /// Create a child region which is indented to the right
    pub fn indent(
        &mut self,
        id_source: impl Hash,
        add_contents: impl FnOnce(&mut Region),
    ) -> InteractInfo {
        assert!(
            self.dir == Direction::Vertical,
            "You can only indent vertical layouts"
        );
        let indent = vec2(self.style.indent, 0.0);
        let child_rect = Rect::from_min_max(self.cursor + indent, self.bottom_right());
        let mut child_region = Region {
            id: self.id.with(id_source),
            align: Align::Min,
            ..self.child_region(child_rect)
        };
        add_contents(&mut child_region);
        let size = child_region.bounding_size();

        // draw a grey line on the left to mark the region
        let line_start = child_rect.min - indent * 0.5;
        let line_start = line_start.round(); // TODO: round to pixel instead
        let line_end = pos2(line_start.x, line_start.y + size.y - 8.0);
        self.add_paint_cmd(PaintCmd::Line {
            points: vec![line_start, line_end],
            color: gray(150, 255),
            width: self.style.line_width,
        });

        self.reserve_space(indent + size, None)
    }

    pub fn left_column(&mut self, width: f32) -> Region {
        self.column(Align::Min, width)
    }

    pub fn centered_column(&mut self, width: f32) -> Region {
        self.column(Align::Center, width)
    }

    pub fn right_column(&mut self, width: f32) -> Region {
        self.column(Align::Max, width)
    }

    /// A column region with a given width.
    pub fn column(&mut self, column_position: Align, width: f32) -> Region {
        let x = match column_position {
            Align::Min => 0.0,
            Align::Center => self.available_width() / 2.0 - width / 2.0,
            Align::Max => self.available_width() - width,
        };
        self.child_region(Rect::from_min_size(
            self.cursor + vec2(x, 0.0),
            vec2(width, self.available_height()),
        ))
    }

    /// Start a region with horizontal layout
    pub fn horizontal(&mut self, add_contents: impl FnOnce(&mut Region)) {
        self.inner_layout(Direction::Horizontal, Align::Min, add_contents)
    }

    /// Start a region with vertical layout
    pub fn vertical(&mut self, add_contents: impl FnOnce(&mut Region)) {
        self.inner_layout(Direction::Vertical, Align::Min, add_contents)
    }

    pub fn inner_layout(
        &mut self,
        dir: Direction,
        align: Align,
        add_contents: impl FnOnce(&mut Self),
    ) {
        let child_rect = Rect::from_min_max(self.cursor, self.bottom_right());
        let mut child_region = Self {
            dir,
            align,
            ..self.child_region(child_rect)
        };
        add_contents(&mut child_region);
        let size = child_region.bounding_size();
        self.reserve_space(size, None);
    }

    /// Temporarily split split a vertical layout into several columns.
    ///
    /// ``` ignore
    /// region.columns(2, |columns| {
    ///     columns[0].add(emigui::widgets::label!("First column"));
    ///     columns[1].add(emigui::widgets::label!("Second column"));
    /// });
    /// ```
    pub fn columns<F, R>(&mut self, num_columns: usize, add_contents: F) -> R
    where
        F: FnOnce(&mut [Self]) -> R,
    {
        // TODO: ensure there is space
        let spacing = self.style.item_spacing.x;
        let total_spacing = spacing * (num_columns as f32 - 1.0);
        let column_width = (self.available_width() - total_spacing) / (num_columns as f32);

        let mut columns: Vec<Self> = (0..num_columns)
            .map(|col_idx| {
                let pos = self.cursor + vec2((col_idx as f32) * (column_width + spacing), 0.0);
                let child_rect =
                    Rect::from_min_max(pos, pos2(pos.x + column_width, self.bottom_right().y));

                Self {
                    id: self.make_child_id(&("column", col_idx)),
                    dir: Direction::Vertical,
                    ..self.child_region(child_rect)
                }
            })
            .collect();

        let result = add_contents(&mut columns[..]);

        let mut sum_width = total_spacing;
        for column in &columns {
            sum_width += column.child_bounds.width();
        }

        let mut max_height = 0.0;
        for region in columns {
            let size = region.bounding_size();
            max_height = size.y.max(max_height);
        }

        let size = vec2(self.available_width().max(sum_width), max_height);
        self.reserve_space(size, None);
        result
    }

    // ------------------------------------------------
}
