use std::collections::HashSet;

use crate::{math::*, types::*};

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Serialize)]
pub struct LayoutOptions {
    /// The width and height of a single character (including any spacing).
    /// All text is monospace!
    pub char_size: Vec2,

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
            char_size: vec2(7.2, 14.0),
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

#[derive(Clone, Debug, Default)]
struct Memory {
    /// The widget being interacted with (e.g. dragged, in case of a slider).
    active_id: Option<Id>,

    /// Which foldable regions are open.
    open_foldables: HashSet<Id>,
}

// ----------------------------------------------------------------------------

struct TextFragment {
    rect: Rect,
    text: String,
}

type TextFragments = Vec<TextFragment>;

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
enum Direction {
    Horizontal,
    Vertical,
}

impl Default for Direction {
    fn default() -> Direction {
        Direction::Vertical
    }
}

// ----------------------------------------------------------------------------

// TODO: give this a better name
#[derive(Clone, Debug, Default)]
struct Layouter {
    /// Doesn't change.
    dir: Direction,

    /// Changes only along self.dir
    cursor: Vec2,

    /// We keep track of our max-size along the orthogonal to self.dir
    size: Vec2,
}

impl Layouter {
    /// Reserve this much space and move the cursor.
    fn reserve_space(&mut self, size: Vec2) {
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
}

// ----------------------------------------------------------------------------

type Id = u64;

#[derive(Clone, Debug, Default)]
pub struct Layout {
    options: LayoutOptions,
    input: GuiInput,
    memory: Memory,
    id: Id,
    layouter: Layouter,
    graphics: Vec<GuiCmd>,
    hovering_graphics: Vec<GuiCmd>,
}

impl Layout {
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
        self.layouter = Default::default();
        self.input = gui_input;
        if !gui_input.mouse_down {
            self.memory.active_id = None;
        }
    }

    // ------------------------------------------------------------------------

    pub fn button<S: Into<String>>(&mut self, text: S) -> InteractInfo {
        let text: String = text.into();
        let id = self.get_id(&text);
        let (text, text_size) = self.layout_text(&text);
        let text_cursor = self.layouter.cursor + self.options.button_padding;
        let (rect, interact) =
            self.reserve_interactive_space(id, text_size + 2.0 * self.options.button_padding);
        self.graphics.push(GuiCmd::Button { interact, rect });
        self.add_text(text_cursor, text);
        interact
    }

    pub fn checkbox<S: Into<String>>(&mut self, text: S, checked: &mut bool) -> InteractInfo {
        let text: String = text.into();
        let id = self.get_id(&text);
        let (text, text_size) = self.layout_text(&text);
        let text_cursor = self.layouter.cursor
            + self.options.button_padding
            + vec2(self.options.start_icon_width, 0.0);
        let (rect, interact) = self.reserve_interactive_space(
            id,
            self.options.button_padding
                + vec2(self.options.start_icon_width, 0.0)
                + text_size
                + self.options.button_padding,
        );
        if interact.clicked {
            *checked = !*checked;
        }
        self.graphics.push(GuiCmd::Checkbox {
            checked: *checked,
            interact,
            rect,
        });
        self.add_text(text_cursor, text);
        interact
    }

    pub fn label<S: Into<String>>(&mut self, text: S) {
        let text: String = text.into();
        let (text, text_size) = self.layout_text(&text);
        self.add_text(self.layouter.cursor, text);
        self.reserve_space_default_spacing(text_size);
    }

    /// A radio button
    pub fn radio<S: Into<String>>(&mut self, text: S, checked: bool) -> InteractInfo {
        let text: String = text.into();
        let id = self.get_id(&text);
        let (text, text_size) = self.layout_text(&text);
        let text_cursor = self.layouter.cursor
            + self.options.button_padding
            + vec2(self.options.start_icon_width, 0.0);
        let (rect, interact) = self.reserve_interactive_space(
            id,
            self.options.button_padding
                + vec2(self.options.start_icon_width, 0.0)
                + text_size
                + self.options.button_padding,
        );
        self.graphics.push(GuiCmd::RadioButton {
            checked,
            interact,
            rect,
        });
        self.add_text(text_cursor, text);
        interact
    }

    pub fn slider_f32<S: Into<String>>(
        &mut self,
        text: S,
        value: &mut f32,
        min: f32,
        max: f32,
    ) -> InteractInfo {
        debug_assert!(min <= max);
        let text: String = text.into();
        let id = self.get_id(&text);
        let (text, text_size) = self.layout_text(&format!("{}: {:.3}", text, value));
        self.add_text(self.layouter.cursor, text);
        self.layouter.reserve_space(text_size);
        let (slider_rect, interact) = self.reserve_interactive_space(
            id,
            Vec2 {
                x: self.options.width,
                y: self.options.char_size.y,
            },
        );

        if interact.active {
            *value = remap_clamp(
                self.input.mouse_pos.x,
                slider_rect.min().x,
                slider_rect.max().x,
                min,
                max,
            );
        }

        self.graphics.push(GuiCmd::Slider {
            interact,
            max,
            min,
            rect: slider_rect,
            value: *value,
        });

        interact
    }

    // ------------------------------------------------------------------------
    // Areas:

    pub fn foldable<S, F>(&mut self, text: S, add_contents: F) -> InteractInfo
    where
        S: Into<String>,
        F: FnOnce(&mut Layout),
    {
        assert!(
            self.layouter.dir == Direction::Vertical,
            "Horizontal foldable is unimplemented"
        );
        let text: String = text.into();
        let id = self.get_id(&text);
        let (text, text_size) = self.layout_text(&text);
        let text_cursor = self.layouter.cursor + self.options.button_padding;
        let (rect, interact) = self.reserve_interactive_space(
            id,
            vec2(
                self.options.width,
                text_size.y + 2.0 * self.options.button_padding.y,
            ),
        );

        if interact.clicked {
            if self.memory.open_foldables.contains(&id) {
                self.memory.open_foldables.remove(&id);
            } else {
                self.memory.open_foldables.insert(id);
            }
        }
        let open = self.memory.open_foldables.contains(&id);

        self.graphics.push(GuiCmd::FoldableHeader {
            interact,
            rect,
            open,
        });
        self.add_text(text_cursor + vec2(self.options.start_icon_width, 0.0), text);

        if open {
            let old_id = self.id;
            self.id = id;
            let old_x = self.layouter.cursor.x;
            self.layouter.cursor.x += self.options.indent;
            add_contents(self);
            self.layouter.cursor.x = old_x;
            self.id = old_id;
        }

        interact
    }

    /// Start a region with horizontal layout
    pub fn horizontal<F>(&mut self, add_contents: F)
    where
        F: FnOnce(&mut Layout),
    {
        let horizontal_layouter = Layouter {
            dir: Direction::Horizontal,
            cursor: self.layouter.cursor,
            ..Default::default()
        };
        let old_layouter = std::mem::replace(&mut self.layouter, horizontal_layouter);
        add_contents(self);
        let horizontal_layouter = std::mem::replace(&mut self.layouter, old_layouter);
        self.layouter.reserve_space(horizontal_layouter.size);
    }

    // ------------------------------------------------------------------------
    // Free painting. It is up to the caller to make sure there is room for these.
    pub fn add_paint_command(&mut self, cmd: GuiCmd) {
        self.graphics.push(cmd);
    }

    // ------------------------------------------------------------------------

    /// Show some text in a window under mouse position.
    pub fn tooltip_text<S: Into<String>>(&mut self, text: S) {
        let window_pos = self.input.mouse_pos + vec2(16.0, 16.0);

        // TODO: less copying
        let mut popup_layout = Layout {
            options: self.options,
            input: self.input,
            memory: self.memory.clone(), // TODO: Arc
            id: self.id,
            layouter: Default::default(),
            graphics: vec![],
            hovering_graphics: vec![],
        };
        popup_layout.layouter.cursor = window_pos + self.options.window_padding;

        popup_layout.label(text);

        // TODO: handle the last item_spacing in a nicer way
        let inner_size = popup_layout.layouter.size - self.options.item_spacing;
        let outer_size = inner_size + 2.0 * self.options.window_padding;

        let rect = Rect::from_min_size(window_pos, outer_size);
        self.hovering_graphics.push(GuiCmd::Window { rect });
        self.hovering_graphics
            .extend(popup_layout.gui_commands().cloned());
    }

    // ------------------------------------------------------------------------

    fn reserve_space_default_spacing(&mut self, size: Vec2) -> Rect {
        let rect = Rect {
            pos: self.layouter.cursor,
            size,
        };
        self.layouter
            .reserve_space(size + self.options.item_spacing);
        rect
    }

    fn reserve_interactive_space(&mut self, id: Id, size: Vec2) -> (Rect, InteractInfo) {
        let rect = self.reserve_space_default_spacing(size);
        let hovered = rect.contains(self.input.mouse_pos);
        let clicked = hovered && self.input.mouse_clicked;
        if clicked {
            self.memory.active_id = Some(id);
        }
        let active = self.memory.active_id == Some(id);

        let interact = InteractInfo {
            hovered,
            clicked,
            active,
        };
        (rect, interact)
    }

    fn get_id(&self, id_str: &str) -> Id {
        use std::hash::Hasher;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        hasher.write_u64(self.id);
        hasher.write(id_str.as_bytes());
        hasher.finish()
    }

    fn layout_text(&self, text: &str) -> (TextFragments, Vec2) {
        let char_size = self.options.char_size;
        let mut cursor_y = 0.0;
        let mut max_width = 0.0;
        let mut text_fragments = Vec::new();
        for line in text.split('\n') {
            // TODO: break long lines
            let line_width = char_size.x * (line.len() as f32);

            text_fragments.push(TextFragment {
                rect: Rect::from_min_size(vec2(0.0, cursor_y), vec2(line_width, char_size.y)),
                text: line.into(),
            });

            cursor_y += char_size.y;
            max_width = line_width.max(max_width);
        }
        let bounding_size = vec2(max_width, cursor_y);
        (text_fragments, bounding_size)
    }

    fn add_text(&mut self, pos: Vec2, text: Vec<TextFragment>) {
        for fragment in text {
            self.graphics.push(GuiCmd::Text {
                pos: pos + vec2(fragment.rect.pos.x, fragment.rect.center().y),
                style: TextStyle::Label,
                text: fragment.text,
            });
        }
    }
}
