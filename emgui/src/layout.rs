use std::collections::HashSet;

use crate::{font::Font, math::*, types::*};

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Serialize)]
pub struct LayoutOptions {
    /// Horizontal and vertical padding within a window frame.
    pub window_padding: Vec2,

    /// Horizontal and vertical spacing between widgets
    pub item_spacing: Vec2,

    /// Indent foldable regions etc by this much.
    pub indent: f32,

    /// Default width of sliders, foldout categories etc. TODO: percentage of parent?
    pub width: f32,

    /// Button size is text size plus this on each side
    pub button_padding: Vec2,

    /// Checkboxed, radio button and foldables have an icon at the start.
    /// The text starts after this many pixels.
    pub start_icon_width: f32,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        LayoutOptions {
            item_spacing: vec2(8.0, 4.0),
            window_padding: vec2(6.0, 6.0),
            indent: 21.0,
            width: 250.0,
            button_padding: vec2(5.0, 3.0),
            start_icon_width: 20.0,
        }
    }
}

// ----------------------------------------------------------------------------

// TODO: rename
pub struct GuiResponse<'a> {
    /// The mouse is hovering above this
    pub hovered: bool,

    /// The mouse went got pressed on this thing this frame
    pub clicked: bool,

    /// The mouse is interacting with this thing (e.g. dragging it)
    pub active: bool,

    /// Used for showing a popup (if any)
    data: &'a mut Data,
}

impl<'a> GuiResponse<'a> {
    /// Show some stuff if the item was hovered
    pub fn tooltip<F>(&mut self, add_contents: F) -> &mut Self
    where
        F: FnOnce(&mut Region),
    {
        if self.hovered {
            let window_pos = self.data.input().mouse_pos + vec2(16.0, 16.0);
            self.data.show_popup(window_pos, add_contents);
        }
        self
    }

    /// Show this text if the item was hovered
    pub fn tooltip_text<S: Into<String>>(&mut self, text: S) -> &mut Self {
        self.tooltip(|popup| {
            popup.label(text);
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

struct TextFragment {
    /// The start of each character, starting at zero.
    x_offsets: Vec<f32>,
    /// 0 for the first line, n * line_spacing for the rest
    y_offset: f32,
    text: String,
}

type TextFragments = Vec<TextFragment>;

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

// ----------------------------------------------------------------------------

type Id = u64;

// TODO: give a better name.
/// Contains the input, options and output of all GUI commands.
#[derive(Clone)]
pub struct Data {
    pub(crate) options: LayoutOptions,
    pub(crate) font: Font, // TODO: Arc?. TODO: move to options.
    pub(crate) input: GuiInput,
    pub(crate) memory: Memory,
    pub(crate) graphics: Vec<GuiCmd>,
    pub(crate) hovering_graphics: Vec<GuiCmd>,
}

impl Data {
    pub fn new(font: Font) -> Data {
        Data {
            options: Default::default(),
            font,
            input: Default::default(),
            memory: Default::default(),
            graphics: Default::default(),
            hovering_graphics: Default::default(),
        }
    }

    pub fn input(&self) -> &GuiInput {
        &self.input
    }

    pub fn gui_commands(&self) -> impl Iterator<Item = &GuiCmd> {
        self.graphics.iter().chain(self.hovering_graphics.iter())
    }

    pub fn options(&self) -> &LayoutOptions {
        &self.options
    }

    pub fn set_options(&mut self, options: LayoutOptions) {
        self.options = options;
    }

    // TODO: move
    pub fn new_frame(&mut self, gui_input: GuiInput) {
        self.graphics.clear();
        self.hovering_graphics.clear();
        self.input = gui_input;
        if !gui_input.mouse_down {
            self.memory.active_id = None;
        }
    }

    /// Show a pop-over window
    pub fn show_popup<F>(&mut self, window_pos: Vec2, add_contents: F)
    where
        F: FnOnce(&mut Region),
    {
        // TODO: nicer way to do layering!
        let num_graphics_before = self.graphics.len();

        let window_padding = self.options.window_padding;

        let mut popup_region = Region {
            data: self,
            id: Default::default(),
            dir: Direction::Vertical,
            cursor: window_pos + window_padding,
            size: vec2(0.0, 0.0),
        };

        add_contents(&mut popup_region);

        // TODO: handle the last item_spacing in a nicer way
        let inner_size = popup_region.size - self.options.item_spacing;
        let outer_size = inner_size + 2.0 * window_padding;

        let rect = Rect::from_min_size(window_pos, outer_size);

        let popup_graphics = self.graphics.split_off(num_graphics_before);
        self.hovering_graphics.push(GuiCmd::Window { rect });
        self.hovering_graphics.extend(popup_graphics);
    }
}

// ----------------------------------------------------------------------------

/// Represents a region of the screen
/// with a type of layout (horizontal or vertical).
pub struct Region<'a> {
    pub(crate) data: &'a mut Data,

    // TODO: add min_size and max_size
    /// Unique ID of this region.
    pub(crate) id: Id,

    /// Doesn't change.
    pub(crate) dir: Direction,

    /// Changes only along self.dir
    pub(crate) cursor: Vec2,

    /// We keep track of our max-size along the orthogonal to self.dir
    pub(crate) size: Vec2,
}

impl<'a> Region<'a> {
    /// It is up to the caller to make sure there is room for this.
    /// Can be used for free painting.
    /// NOTE: all coordinates are screen coordinates!
    pub fn add_graphic(&mut self, gui_cmd: GuiCmd) {
        self.data.graphics.push(gui_cmd)
    }

    pub fn options(&self) -> &LayoutOptions {
        self.data.options()
    }

    pub fn set_options(&mut self, options: LayoutOptions) {
        self.data.set_options(options)
    }

    pub fn input(&self) -> &GuiInput {
        self.data.input()
    }

    pub fn button<S: Into<String>>(&mut self, text: S) -> GuiResponse {
        let text: String = text.into();
        let id = self.get_id(&text);
        let (text, text_size) = self.layout_text(&text);
        let text_cursor = self.cursor + self.options().button_padding;
        let (rect, interact) =
            self.reserve_space(text_size + 2.0 * self.options().button_padding, Some(id));
        self.add_graphic(GuiCmd::Button { interact, rect });
        self.add_text(text_cursor, text);
        self.response(interact)
    }

    pub fn checkbox<S: Into<String>>(&mut self, text: S, checked: &mut bool) -> GuiResponse {
        let text: String = text.into();
        let id = self.get_id(&text);
        let (text, text_size) = self.layout_text(&text);
        let text_cursor = self.cursor
            + self.options().button_padding
            + vec2(self.options().start_icon_width, 0.0);
        let (rect, interact) = self.reserve_space(
            self.options().button_padding
                + vec2(self.options().start_icon_width, 0.0)
                + text_size
                + self.options().button_padding,
            Some(id),
        );
        if interact.clicked {
            *checked = !*checked;
        }
        self.add_graphic(GuiCmd::Checkbox {
            checked: *checked,
            interact,
            rect,
        });
        self.add_text(text_cursor, text);
        self.response(interact)
    }

    pub fn label<S: Into<String>>(&mut self, text: S) -> GuiResponse {
        let text: String = text.into();
        let (text, text_size) = self.layout_text(&text);
        self.add_text(self.cursor, text);
        let (_, interact) = self.reserve_space(text_size, None);
        self.response(interact)
    }

    /// A radio button
    pub fn radio<S: Into<String>>(&mut self, text: S, checked: bool) -> GuiResponse {
        let text: String = text.into();
        let id = self.get_id(&text);
        let (text, text_size) = self.layout_text(&text);
        let text_cursor = self.cursor
            + self.options().button_padding
            + vec2(self.options().start_icon_width, 0.0);
        let (rect, interact) = self.reserve_space(
            self.options().button_padding
                + vec2(self.options().start_icon_width, 0.0)
                + text_size
                + self.options().button_padding,
            Some(id),
        );
        self.add_graphic(GuiCmd::RadioButton {
            checked,
            interact,
            rect,
        });
        self.add_text(text_cursor, text);
        self.response(interact)
    }

    pub fn slider_f32<S: Into<String>>(
        &mut self,
        text: S,
        value: &mut f32,
        min: f32,
        max: f32,
    ) -> GuiResponse {
        debug_assert!(min <= max);
        let text: String = text.into();
        let id = self.get_id(&text);
        let (text, text_size) = self.layout_text(&format!("{}: {:.3}", text, value));
        self.add_text(self.cursor, text);
        self.reserve_space_inner(text_size);
        let (slider_rect, interact) = self.reserve_space(
            Vec2 {
                x: self.options().width,
                y: self.data.font.line_spacing(),
            },
            Some(id),
        );

        if interact.active {
            *value = remap_clamp(
                self.input().mouse_pos.x,
                slider_rect.min().x,
                slider_rect.max().x,
                min,
                max,
            );
        }

        self.add_graphic(GuiCmd::Slider {
            interact,
            max,
            min,
            rect: slider_rect,
            value: *value,
        });

        self.response(interact)
    }

    // ------------------------------------------------------------------------
    // Areas:

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
        let id = self.get_id(&text);
        let (text, text_size) = self.layout_text(&text);
        let text_cursor = self.cursor + self.options().button_padding;
        let (rect, interact) = self.reserve_space(
            vec2(
                self.options().width,
                text_size.y + 2.0 * self.options().button_padding.y,
            ),
            Some(id),
        );

        if interact.clicked {
            if self.data.memory.open_foldables.contains(&id) {
                self.data.memory.open_foldables.remove(&id);
            } else {
                self.data.memory.open_foldables.insert(id);
            }
        }
        let open = self.data.memory.open_foldables.contains(&id);

        self.add_graphic(GuiCmd::FoldableHeader {
            interact,
            rect,
            open,
        });
        self.add_text(
            text_cursor + vec2(self.options().start_icon_width, 0.0),
            text,
        );

        if open {
            // TODO: new region
            let old_id = self.id;
            self.id = id;
            let old_x = self.cursor.x;
            self.cursor.x += self.options().indent;
            add_contents(self);
            self.cursor.x = old_x;
            self.id = old_id;
        }

        self.response(interact)
    }

    /// Start a region with horizontal layout
    pub fn horizontal<F>(&mut self, add_contents: F)
    where
        F: FnOnce(&mut Region),
    {
        let mut horizontal_region = Region {
            data: self.data,
            id: self.id,
            dir: Direction::Horizontal,
            cursor: self.cursor,
            size: vec2(0.0, 0.0),
        };
        add_contents(&mut horizontal_region);
        let size = horizontal_region.size;
        self.reserve_space_inner(size);
    }

    // ------------------------------------------------------------------------

    fn reserve_space(&mut self, size: Vec2, interaction_id: Option<Id>) -> (Rect, InteractInfo) {
        let rect = Rect {
            pos: self.cursor,
            size,
        };
        self.reserve_space_inner(size + self.options().item_spacing);
        let hovered = rect.contains(self.input().mouse_pos);
        let clicked = hovered && self.input().mouse_clicked;
        let active = if interaction_id.is_some() {
            if clicked {
                self.data.memory.active_id = interaction_id;
            }
            self.data.memory.active_id == interaction_id
        } else {
            false
        };

        let interact = InteractInfo {
            hovered,
            clicked,
            active,
        };
        (rect, interact)
    }

    /// Reserve this much space and move the cursor.
    fn reserve_space_inner(&mut self, size: Vec2) {
        if self.dir == Direction::Horizontal {
            self.cursor.x += size.x;
            self.size.x += size.x;
            self.size.y = self.size.y.max(size.y);
        } else {
            self.cursor.y += size.y;
            self.size.y += size.y;
            self.size.x = self.size.x.max(size.x);
        }
    }

    fn get_id(&self, id_str: &str) -> Id {
        use std::hash::Hasher;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        hasher.write_u64(self.id);
        hasher.write(id_str.as_bytes());
        hasher.finish()
    }

    // TODO: move this function
    fn layout_text(&self, text: &str) -> (TextFragments, Vec2) {
        let line_spacing = self.data.font.line_spacing();
        let mut cursor_y = 0.0;
        let mut max_width = 0.0;
        let mut text_fragments = Vec::new();
        for line in text.split('\n') {
            let x_offsets = self.data.font.layout_single_line(&line);
            let line_width = *x_offsets.last().unwrap();
            text_fragments.push(TextFragment {
                x_offsets,
                y_offset: cursor_y,
                text: line.into(),
            });

            cursor_y += line_spacing;
            max_width = line_width.max(max_width);
        }
        let bounding_size = vec2(max_width, cursor_y);
        (text_fragments, bounding_size)
    }

    fn add_text(&mut self, pos: Vec2, text: Vec<TextFragment>) {
        for fragment in text {
            self.add_graphic(GuiCmd::Text {
                pos: pos + vec2(0.0, fragment.y_offset),
                style: TextStyle::Label,
                text: fragment.text,
                x_offsets: fragment.x_offsets,
            });
        }
    }

    fn response(&mut self, interact: InteractInfo) -> GuiResponse {
        GuiResponse {
            hovered: interact.hovered,
            clicked: interact.clicked,
            active: interact.active,
            data: self.data,
        }
    }
}
