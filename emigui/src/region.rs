use std::{hash::Hash, sync::Arc};

use crate::{color::*, font::TextFragment, layout::*, widgets::*, *};

/// Represents a region of the screen
/// with a type of layout (horizontal or vertical).
/// TODO: make Region a trait so we can have type-safe HorizontalRegion etc?
pub struct Region {
    // TODO: remove pub(crate) from all members.
    //
    /// How we access input, output and memory
    pub(crate) ctx: Arc<Context>,

    /// ID of this region.
    /// Generated based on id of parent region together with
    /// another source of child identity (e.g. window title).
    /// Acts like a namespace for child regions.
    /// Hopefully unique.
    pub(crate) id: Id,

    /// Where to put the graphics output of this Region
    pub(crate) layer: Layer,

    /// Everything painte in this rect will be clipped against this.
    /// This means nothing outside of this rectangle will be visible on screen.
    pub(crate) clip_rect: Rect,

    /// The `rect` represents where in space the region is
    /// and its max size (original available_space).
    /// Note that the size may be infinite in one or both dimensions.
    /// The widgets will TRY to fit within the rect,
    /// but may overflow (which you will see in bounding_size).
    pub(crate) desired_rect: Rect, // TODO: rename desired_rect

    /// Bounding box of children.
    /// We keep track of our max-size along the orthogonal to self.dir
    /// Initially set to zero.
    /// TODO: make into `child_bounds: Rect`
    pub(crate) bounding_size: Vec2,

    /// Overide default style in this region
    pub(crate) style: Style,

    // Layout stuff follows. TODO: move to own type and abstract.
    /// Doesn't change.
    pub(crate) dir: Direction,

    pub(crate) align: Align,

    /// Where the next widget will be put.
    /// Progresses along self.dir.
    /// Initially set to rect.min()
    pub(crate) cursor: Pos2,
}

impl Region {
    pub fn new(ctx: Arc<Context>, layer: Layer, id: Id, rect: Rect) -> Self {
        let style = ctx.style();
        Region {
            ctx,
            id,
            layer,
            clip_rect: rect,
            desired_rect: rect,
            bounding_size: Vec2::default(),
            style,
            cursor: rect.min(),
            dir: Direction::Vertical,
            align: Align::Min,
        }
    }

    pub fn child_region(&self, child_rect: Rect) -> Self {
        // Allow child widgets to be just on the border and still have an outline with some thickness
        const CLIP_RECT_MARGIN: f32 = 3.0;
        Region {
            ctx: self.ctx.clone(),
            layer: self.layer,
            style: self.style,
            id: self.id,
            clip_rect: self
                .clip_rect
                .intersect(child_rect.expand(CLIP_RECT_MARGIN)),
            desired_rect: child_rect,
            cursor: child_rect.min(),
            bounding_size: vec2(0.0, 0.0),
            dir: self.dir,
            align: self.align,
        }
    }

    /// It is up to the caller to make sure there is room for this.
    /// Can be used for free painting.
    /// NOTE: all coordinates are screen coordinates!
    pub fn add_paint_cmd(&mut self, paint_cmd: PaintCmd) {
        self.ctx
            .graphics
            .lock()
            .layer(self.layer)
            .push((self.clip_rect(), paint_cmd))
    }

    pub fn add_paint_cmds(&mut self, mut cmds: Vec<PaintCmd>) {
        let clip_rect = self.clip_rect();
        self.ctx
            .graphics
            .lock()
            .layer(self.layer)
            .extend(cmds.drain(..).map(|cmd| (clip_rect, cmd)));
    }

    pub fn round_to_pixel(&self, point: f32) -> f32 {
        self.ctx.round_to_pixel(point)
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

    pub fn fonts(&self) -> &Fonts {
        &*self.ctx.fonts
    }

    /// Screen-space rectangle for clipping what we paint in this region.
    /// This is used, for instance, to avoid painting outside a window that is smaller
    /// than its contents.
    pub fn clip_rect(&self) -> Rect {
        self.clip_rect
    }

    pub fn available_width(&self) -> f32 {
        self.desired_rect.right() - self.cursor.x
    }

    pub fn available_height(&self) -> f32 {
        self.desired_rect.bottom() - self.cursor.y
    }

    /// This how much more space we can take up without overflowing our parent.
    /// Shrinks as cursor increments.
    pub fn available_space(&self) -> Vec2 {
        self.desired_rect.max() - self.cursor
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
    // Sub-regions:

    /// Create a child region which is indented to the right
    pub fn indent<F>(&mut self, id: Id, add_contents: F)
    where
        F: FnOnce(&mut Region),
    {
        assert!(
            self.dir == Direction::Vertical,
            "You can only indent vertical layouts"
        );
        let indent = vec2(self.style.indent, 0.0);
        let child_rect = Rect::from_min_max(self.cursor + indent, self.desired_rect.max());
        let mut child_region = Region {
            id,
            align: Align::Min,
            ..self.child_region(child_rect)
        };
        add_contents(&mut child_region);
        let size = child_region.bounding_size;

        // draw a grey line on the left to mark the region
        let line_start = child_rect.min() - indent * 0.5;
        let line_start = line_start.round(); // TODO: round to pixel instead
        let line_end = pos2(line_start.x, line_start.y + size.y - 8.0);
        self.add_paint_cmd(PaintCmd::Line {
            points: vec![line_start, line_end],
            color: gray(150, 255),
            width: self.style.line_width,
        });

        self.reserve_space_without_padding(indent + size);
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

    pub fn inner_layout<F>(&mut self, dir: Direction, align: Align, add_contents: F)
    where
        F: FnOnce(&mut Region),
    {
        let child_rect = Rect::from_min_max(self.cursor, self.desired_rect.max());
        let mut child_region = Region {
            dir,
            align,
            ..self.child_region(child_rect)
        };
        add_contents(&mut child_region);
        let size = child_region.bounding_size;
        self.reserve_space_without_padding(size);
    }

    /// Start a region with horizontal layout
    pub fn horizontal<F>(&mut self, align: Align, add_contents: F)
    where
        F: FnOnce(&mut Region),
    {
        self.inner_layout(Direction::Horizontal, align, add_contents)
    }

    /// Start a region with vertical layout
    pub fn vertical<F>(&mut self, align: Align, add_contents: F)
    where
        F: FnOnce(&mut Region),
    {
        self.inner_layout(Direction::Vertical, align, add_contents)
    }

    /// Temporarily split split a vertical layout into several columns.
    ///
    /// region.columns(2, |columns| {
    ///     columns[0].add(emigui::widgets::label!("First column"));
    ///     columns[1].add(emigui::widgets::label!("Second column"));
    /// });
    pub fn columns<F, R>(&mut self, num_columns: usize, add_contents: F) -> R
    where
        F: FnOnce(&mut [Region]) -> R,
    {
        // TODO: ensure there is space
        let padding = self.style.item_spacing.x;
        let total_padding = padding * (num_columns as f32 - 1.0);
        let column_width = (self.available_width() - total_padding) / (num_columns as f32);

        let mut columns: Vec<Region> = (0..num_columns)
            .map(|col_idx| {
                let pos = self.cursor + vec2((col_idx as f32) * (column_width + padding), 0.0);
                let child_rect =
                    Rect::from_min_max(pos, pos2(pos.x + column_width, self.desired_rect.bottom()));

                Region {
                    id: self.make_child_id(&("column", col_idx)),
                    dir: Direction::Vertical,
                    ..self.child_region(child_rect)
                }
            })
            .collect();

        let result = add_contents(&mut columns[..]);

        let mut max_height = 0.0;
        for region in columns {
            let size = region.bounding_size;
            max_height = size.y.max(max_height);
        }

        self.reserve_space_without_padding(vec2(self.available_width(), max_height));
        result
    }

    // ------------------------------------------------------------------------

    pub fn add<W: Widget>(&mut self, widget: W) -> GuiResponse {
        widget.add_to(self)
    }

    // Convenience functions:

    pub fn add_label(&mut self, text: impl Into<String>) -> GuiResponse {
        self.add(Label::new(text))
    }

    pub fn add_hyperlink(&mut self, url: impl Into<String>) -> GuiResponse {
        self.add(Hyperlink::new(url))
    }

    pub fn collapsing<S, F>(&mut self, text: S, add_contents: F) -> GuiResponse
    where
        S: Into<String>,
        F: FnOnce(&mut Region),
    {
        CollapsingHeader::new(text).show(self, add_contents)
    }

    // ------------------------------------------------------------------------

    pub fn reserve_space(&mut self, size: Vec2, interaction_id: Option<Id>) -> InteractInfo {
        let pos = self.reserve_space_without_padding(size + self.style.item_spacing);
        let rect = Rect::from_min_size(pos, size);
        self.ctx.interact(self.layer, rect, interaction_id)
    }

    /// Reserve this much space and move the cursor.
    /// Returns where to put the widget.
    pub fn reserve_space_without_padding(&mut self, size: Vec2) -> Pos2 {
        let mut pos = self.cursor;
        if self.dir == Direction::Horizontal {
            pos.y += match self.align {
                Align::Min => 0.0,
                Align::Center => 0.5 * (self.available_height() - size.y),
                Align::Max => self.available_height() - size.y,
            };
            self.cursor.x += size.x;
            self.bounding_size.x += size.x;
            self.bounding_size.y = self.bounding_size.y.max(size.y);
        } else {
            pos.x += match self.align {
                Align::Min => 0.0,
                Align::Center => 0.5 * (self.available_width() - size.x),
                Align::Max => self.available_width() - size.x,
            };
            self.cursor.y += size.y;
            self.bounding_size.y += size.y;
            self.bounding_size.x = self.bounding_size.x.max(size.x);
        }
        pos
    }

    /// Will warn if the returned id is not guaranteed unique.
    /// Use this to generate widget ids for widgets that have persistent state in Memory.
    /// If the child_id_source is not unique within this region
    /// then an error will be printed at the current cursor position.
    pub fn make_unique_id<IdSource>(&self, child_id_source: &IdSource) -> Id
    where
        IdSource: Hash + std::fmt::Debug,
    {
        let id = self.id.with(child_id_source);
        self.ctx
            .register_unique_id(id, child_id_source, self.cursor)
    }

    /// Make an Id that is unique to this positon.
    /// Can be used for widgets that do NOT persist state in Memory
    /// but you still need to interact with (e.g. buttons, sliders).
    pub fn make_position_id(&self) -> Id {
        self.id.with(&Id::from_pos(self.cursor))
    }

    pub fn make_child_id<H: Hash>(&self, child_id: &H) -> Id {
        self.id.with(child_id)
    }

    /// Show some text anywhere in the region.
    /// To center the text at the given position, use `align: (Center, Center)`.
    /// If you want to draw text floating on top of everything,
    /// consider using Context.floating_text instead.
    pub fn floating_text(
        &mut self,
        pos: Pos2,
        text: &str,
        text_style: TextStyle,
        align: (Align, Align),
        text_color: Option<Color>,
    ) -> Vec2 {
        let font = &self.fonts()[text_style];
        let (text, size) = font.layout_multiline(text, std::f32::INFINITY);
        let rect = align_rect(Rect::from_min_size(pos, size), align);
        self.add_text(rect.min(), text_style, text, text_color);
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
}
