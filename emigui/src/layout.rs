use std::{
    collections::HashSet,
    hash::Hash,
    sync::{Arc, Mutex},
};

use crate::{
    font::TextFragment,
    fonts::{Fonts, TextStyle},
    math::*,
    types::*,
    widgets::{label, Widget},
};

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Serialize)]
pub struct LayoutOptions {
    /// Horizontal and vertical padding within a window frame.
    pub window_padding: Vec2,

    /// Button size is text size plus this on each side
    pub button_padding: Vec2,

    /// Horizontal and vertical spacing between widgets
    pub item_spacing: Vec2,

    /// Indent foldable regions etc by this much.
    pub indent: f32,

    /// Anything clickable is (at least) this wide.
    pub clickable_diameter: f32,

    /// Checkboxes, radio button and foldables have an icon at the start.
    /// The text starts after this many pixels.
    pub start_icon_width: f32,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        LayoutOptions {
            window_padding: vec2(6.0, 6.0),
            button_padding: vec2(5.0, 3.0),
            item_spacing: vec2(8.0, 4.0),
            indent: 21.0,
            clickable_diameter: 34.0,
            start_icon_width: 20.0,
        }
    }
}

// ----------------------------------------------------------------------------

// TODO: rename GuiResponse
pub struct GuiResponse {
    /// The mouse is hovering above this
    pub hovered: bool,

    /// The mouse went got pressed on this thing this frame
    pub clicked: bool,

    /// The mouse is interacting with this thing (e.g. dragging it)
    pub active: bool,

    /// The region of the screen we are talking about
    pub rect: Rect,

    /// Used for showing a popup (if any)
    data: Arc<Data>,
}

impl GuiResponse {
    /// Show some stuff if the item was hovered
    pub fn tooltip<F>(&mut self, add_contents: F) -> &mut Self
    where
        F: FnOnce(&mut Region),
    {
        if self.hovered {
            let window_pos = self.data.input().mouse_pos + vec2(16.0, 16.0);
            show_popup(&self.data, window_pos, add_contents);
        }
        self
    }

    /// Show this text if the item was hovered
    pub fn tooltip_text<S: Into<String>>(&mut self, text: S) -> &mut Self {
        self.tooltip(|popup| {
            popup.add(label(text));
        })
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct Memory {
    /// The widget being interacted with (e.g. dragged, in case of a slider).
    active_id: Option<Id>,

    /// Which foldable regions are open.
    open_foldables: HashSet<Id>,
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

impl Default for Direction {
    fn default() -> Direction {
        Direction::Vertical
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Align {
    /// Left/Top
    Min,

    /// Note: requires a bounded/known available_width.
    Center,

    /// Right/Bottom
    /// Note: requires a bounded/known available_width.
    Max,
}

impl Default for Align {
    fn default() -> Align {
        Align::Min
    }
}

// ----------------------------------------------------------------------------

pub type Id = u64;

pub fn make_id<H: Hash>(source: &H) -> Id {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

// ----------------------------------------------------------------------------

/// TODO: improve this
#[derive(Clone, Default)]
pub struct GraphicLayers {
    pub(crate) graphics: Vec<GuiCmd>,
    pub(crate) hovering_graphics: Vec<GuiCmd>,
}

impl GraphicLayers {
    pub fn drain(&mut self) -> impl ExactSizeIterator<Item = GuiCmd> {
        // TODO: there must be a nicer way to do this?
        let mut all_commands: Vec<_> = self.graphics.drain(..).collect();
        all_commands.extend(self.hovering_graphics.drain(..));
        all_commands.into_iter()
    }
}

// ----------------------------------------------------------------------------

// TODO: give a better name.
/// Contains the input, options and output of all GUI commands.
pub struct Data {
    /// The default options for new regions
    pub(crate) options: Mutex<LayoutOptions>,
    pub(crate) fonts: Arc<Fonts>,
    pub(crate) input: GuiInput,
    pub(crate) memory: Mutex<Memory>,
    pub(crate) graphics: Mutex<GraphicLayers>,
}

impl Clone for Data {
    fn clone(&self) -> Self {
        Data {
            options: Mutex::new(self.options()),
            fonts: self.fonts.clone(),
            input: self.input,
            memory: Mutex::new(self.memory.lock().unwrap().clone()),
            graphics: Mutex::new(self.graphics.lock().unwrap().clone()),
        }
    }
}

impl Data {
    pub fn new(pixels_per_point: f32) -> Data {
        Data {
            options: Default::default(),
            fonts: Arc::new(Fonts::new(pixels_per_point)),
            input: Default::default(),
            memory: Default::default(),
            graphics: Default::default(),
        }
    }

    pub fn input(&self) -> &GuiInput {
        &self.input
    }

    pub fn options(&self) -> LayoutOptions {
        *self.options.lock().unwrap()
    }

    pub fn set_options(&self, options: LayoutOptions) {
        *self.options.lock().unwrap() = options;
    }

    // TODO: move
    pub fn new_frame(&mut self, gui_input: GuiInput) {
        self.input = gui_input;
        if !gui_input.mouse_down {
            self.memory.lock().unwrap().active_id = None;
        }
    }
}

/// Show a pop-over window
pub fn show_popup<F>(data: &Arc<Data>, window_pos: Vec2, add_contents: F)
where
    F: FnOnce(&mut Region),
{
    // TODO: nicer way to do layering!
    let num_graphics_before = data.graphics.lock().unwrap().graphics.len();

    let options = data.options();
    let window_padding = options.window_padding;

    let mut popup_region = Region {
        data: data.clone(),
        options,
        id: Default::default(),
        dir: Direction::Vertical,
        align: Align::Min,
        cursor: window_pos + window_padding,
        bounding_size: vec2(0.0, 0.0),
        available_space: vec2(data.input.screen_size.x.min(350.0), std::f32::INFINITY), // TODO: popup/tooltip width
    };

    add_contents(&mut popup_region);

    // TODO: handle the last item_spacing in a nicer way
    let inner_size = popup_region.bounding_size - options.item_spacing;
    let outer_size = inner_size + 2.0 * window_padding;

    let rect = Rect::from_min_size(window_pos, outer_size);

    let mut graphics = data.graphics.lock().unwrap();
    let popup_graphics = graphics.graphics.split_off(num_graphics_before);
    graphics.hovering_graphics.push(GuiCmd::Window { rect });
    graphics.hovering_graphics.extend(popup_graphics);
}

// ----------------------------------------------------------------------------

/// Represents a region of the screen
/// with a type of layout (horizontal or vertical).
/// TODO: make Region a trait so we can have type-safe HorizontalRegion etc?
pub struct Region {
    pub(crate) data: Arc<Data>,

    pub(crate) options: LayoutOptions,

    /// Unique ID of this region.
    pub(crate) id: Id,

    /// Doesn't change.
    pub(crate) dir: Direction,

    pub(crate) align: Align,

    /// Changes only along self.dir
    pub(crate) cursor: Vec2,

    /// Bounding box children.
    /// We keep track of our max-size along the orthogonal to self.dir
    pub(crate) bounding_size: Vec2,

    /// This how much space we can take up without overflowing our parent.
    /// Shrinks as cursor increments.
    pub(crate) available_space: Vec2,
}

impl Region {
    /// It is up to the caller to make sure there is room for this.
    /// Can be used for free painting.
    /// NOTE: all coordinates are screen coordinates!
    pub fn add_graphic(&mut self, gui_cmd: GuiCmd) {
        self.data.graphics.lock().unwrap().graphics.push(gui_cmd)
    }

    /// Options for this region, and any child regions we may spawn.
    pub fn options(&self) -> &LayoutOptions {
        &self.options
    }

    pub fn data(&self) -> &Arc<Data> {
        &self.data
    }

    pub fn input(&self) -> &GuiInput {
        self.data.input()
    }

    pub fn fonts(&self) -> &Fonts {
        &*self.data.fonts
    }

    pub fn width(&self) -> f32 {
        self.available_space.x
    }

    pub fn direction(&self) -> Direction {
        self.dir
    }

    pub fn cursor(&self) -> Vec2 {
        self.cursor
    }

    // ------------------------------------------------------------------------
    // Sub-regions:

    pub fn foldable<S, F>(&mut self, text: S, add_contents: F) -> GuiResponse
    where
        S: Into<String>,
        F: FnOnce(&mut Region),
    {
        assert!(
            self.dir == Direction::Vertical,
            "Horizontal foldable is unimplemented"
        );
        let text: String = text.into();
        let id = self.make_child_id(&text);
        let text_style = TextStyle::Button;
        let font = &self.fonts()[text_style];
        let (text, text_size) = font.layout_multiline(&text, self.width());
        let text_cursor = self.cursor + self.options().button_padding;
        let interact = self.reserve_space(
            vec2(
                self.available_space.x,
                text_size.y + 2.0 * self.options().button_padding.y,
            ),
            Some(id),
        );

        let open = {
            let mut memory = self.data.memory.lock().unwrap();
            if interact.clicked {
                if memory.open_foldables.contains(&id) {
                    memory.open_foldables.remove(&id);
                } else {
                    memory.open_foldables.insert(id);
                }
            }
            memory.open_foldables.contains(&id)
        };

        self.add_graphic(GuiCmd::FoldableHeader { interact, open });
        self.add_text(
            text_cursor + vec2(self.options().start_icon_width, 0.0),
            text_style,
            text,
            None,
        );

        if open {
            let old_id = self.id;
            self.id = id;
            self.indent(add_contents);
            self.id = old_id;
        }

        self.response(interact)
    }

    /// Create a child region which is indented to the right
    pub fn indent<F>(&mut self, add_contents: F)
    where
        F: FnOnce(&mut Region),
    {
        let indent = vec2(self.options().indent, 0.0);
        let mut child_region = Region {
            data: self.data.clone(),
            options: self.options,
            id: self.id,
            dir: self.dir,
            align: Align::Min,
            cursor: self.cursor + indent,
            bounding_size: vec2(0.0, 0.0),
            available_space: self.available_space - indent,
        };
        add_contents(&mut child_region);
        let size = child_region.bounding_size;
        self.reserve_space_without_padding(indent + size);
    }

    /// A horizontally centered region of the given width.
    pub fn centered_column(&mut self, width: f32, align: Align) -> Region {
        Region {
            data: self.data.clone(),
            options: self.options,
            id: self.id,
            dir: self.dir,
            cursor: vec2((self.available_space.x - width) / 2.0, self.cursor.y),
            align,
            bounding_size: vec2(0.0, 0.0),
            available_space: vec2(width, self.available_space.y),
        }
    }

    /// Start a region with horizontal layout
    pub fn horizontal<F>(&mut self, align: Align, add_contents: F)
    where
        F: FnOnce(&mut Region),
    {
        let mut child_region = Region {
            data: self.data.clone(),
            options: self.options,
            id: self.id,
            dir: Direction::Horizontal,
            align,
            cursor: self.cursor,
            bounding_size: vec2(0.0, 0.0),
            available_space: self.available_space,
        };
        add_contents(&mut child_region);
        let size = child_region.bounding_size;
        self.reserve_space_without_padding(size);
    }

    /// Temporarily split split a vertical layout into several columns.
    ///
    /// region.columns(2, |columns| {
    ///     columns[0].add(emigui::widgets::label("First column"));
    ///     columns[1].add(emigui::widgets::label("Second column"));
    /// });
    pub fn columns<F, R>(&mut self, num_columns: usize, add_contents: F) -> R
    where
        F: FnOnce(&mut [Region]) -> R,
    {
        // TODO: ensure there is space
        let padding = self.options().item_spacing.x;
        let total_padding = padding * (num_columns as f32 - 1.0);
        let column_width = (self.available_space.x - total_padding) / (num_columns as f32);

        let mut columns: Vec<Region> = (0..num_columns)
            .map(|col_idx| Region {
                data: self.data.clone(),
                options: self.options,
                id: self.make_child_id(&("column", col_idx)),
                dir: Direction::Vertical,
                align: self.align,
                cursor: self.cursor + vec2((col_idx as f32) * (column_width + padding), 0.0),
                bounding_size: vec2(0.0, 0.0),
                available_space: vec2(column_width, self.available_space.y),
            })
            .collect();

        let result = add_contents(&mut columns[..]);

        let mut max_height = 0.0;
        for region in columns {
            let size = region.bounding_size;
            max_height = size.y.max(max_height);
        }

        self.reserve_space_without_padding(vec2(self.available_space.x, max_height));
        result
    }

    // ------------------------------------------------------------------------

    pub fn add<W: Widget>(&mut self, widget: W) -> GuiResponse {
        widget.add_to(self)
    }

    // ------------------------------------------------------------------------

    pub fn reserve_space(&mut self, size: Vec2, interaction_id: Option<Id>) -> InteractInfo {
        let pos = self.reserve_space_without_padding(size + self.options().item_spacing);
        let rect = Rect::from_min_size(pos, size);
        let mut memory = self.data.memory.lock().unwrap();

        let is_something_else_active =
            memory.active_id.is_some() && memory.active_id != interaction_id;

        let hovered = !is_something_else_active && rect.contains(self.input().mouse_pos);
        let clicked = hovered && self.input().mouse_clicked;
        let active = if interaction_id.is_some() {
            if clicked {
                memory.active_id = interaction_id;
            }
            memory.active_id == interaction_id
        } else {
            false
        };

        InteractInfo {
            rect,
            hovered,
            clicked,
            active,
        }
    }

    /// Reserve this much space and move the cursor.
    pub fn reserve_space_without_padding(&mut self, size: Vec2) -> Vec2 {
        let mut pos = self.cursor;
        if self.dir == Direction::Horizontal {
            pos.y += match self.align {
                Align::Min => 0.0,
                Align::Center => 0.5 * (self.available_space.y - size.y),
                Align::Max => self.available_space.y - size.y,
            };
            self.cursor.x += size.x;
            self.available_space.x -= size.x;
            self.bounding_size.x += size.x;
            self.bounding_size.y = self.bounding_size.y.max(size.y);
        } else {
            pos.x += match self.align {
                Align::Min => 0.0,
                Align::Center => 0.5 * (self.available_space.x - size.x),
                Align::Max => self.available_space.x - size.x,
            };
            self.cursor.y += size.y;
            self.available_space.y -= size.x;
            self.bounding_size.y += size.y;
            self.bounding_size.x = self.bounding_size.x.max(size.x);
        }
        pos
    }

    pub fn make_child_id<H: Hash>(&self, child_id: &H) -> Id {
        use std::hash::Hasher;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        hasher.write_u64(self.id);
        child_id.hash(&mut hasher);
        hasher.finish()
    }

    pub fn combined_id(&self, child_id: Option<Id>) -> Option<Id> {
        child_id.map(|child_id| {
            use std::hash::Hasher;
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            hasher.write_u64(self.id);
            child_id.hash(&mut hasher);
            hasher.finish()
        })
    }

    pub fn add_text(
        &mut self,
        pos: Vec2,
        text_style: TextStyle,
        text: Vec<TextFragment>,
        color: Option<Color>,
    ) {
        for fragment in text {
            self.add_graphic(GuiCmd::Text {
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
            data: self.data.clone(),
        }
    }
}
