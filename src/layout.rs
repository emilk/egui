use std::collections::HashSet;

use crate::{math::*, types::*};

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Serialize)]
pub struct LayoutOptions {
    /// The width and height of a single character (including any spacing).
    /// All text is monospace!
    pub char_size: Vec2,

    // Horizontal and vertical spacing between widgets
    pub item_spacing: Vec2,

    /// Indent foldable regions etc by this much.
    pub indent: f32,

    /// Default width of sliders, foldout categories etc. TODO: percentage of parent?
    pub width: f32,

    /// Button size is text size plus this on each side
    pub button_padding: Vec2,

    /// Height of a checkbox and radio button
    pub checkbox_radio_height: f32,

    /// Height of a slider
    pub slider_height: f32,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        LayoutOptions {
            char_size: vec2(7.2, 14.0),
            item_spacing: vec2(5.0, 3.0),
            indent: 21.0,
            width: 200.0,
            button_padding: vec2(8.0, 8.0),
            checkbox_radio_height: 24.0,
            slider_height: 32.0,
        }
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct Memory {
    /// The widget being interacted with (e.g. dragged, in case of a slider).
    pub active_id: Option<Id>,

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

type Id = u64;

#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub layout_options: LayoutOptions,
    pub input: GuiInput,
    pub cursor: Vec2,
    id: Id,
    pub memory: Memory,
    pub commands: Vec<GuiCmd>,
}

impl Layout {
    pub fn input(&self) -> &GuiInput {
        &self.input
    }

    pub fn gui_commands(&self) -> &[GuiCmd] {
        &self.commands
    }

    // ------------------------------------------------------------------------

    pub fn button<S: Into<String>>(&mut self, text: S) -> InteractInfo {
        let text: String = text.into();
        let id = self.get_id(&text);
        let (text, text_size) = self.layout_text(&text);
        let text_cursor = self.cursor + self.layout_options.button_padding;
        let (rect, interact) =
            self.reserve_space(id, text_size + 2.0 * self.layout_options.button_padding);
        self.commands.push(GuiCmd::Button { interact, rect });
        self.add_text(text_cursor, text);
        interact
    }

    pub fn checkbox<S: Into<String>>(&mut self, label: S, checked: &mut bool) -> InteractInfo {
        let label: String = label.into();
        let id = self.get_id(&label);
        let (rect, interact) = self.reserve_space(
            id,
            Vec2 {
                x: self.layout_options.width,
                y: self.layout_options.checkbox_radio_height,
            },
        );
        if interact.clicked {
            *checked = !*checked;
        }
        self.commands.push(GuiCmd::Checkbox {
            checked: *checked,
            interact,
            rect,
            text: label,
        });
        interact
    }

    pub fn label<S: Into<String>>(&mut self, text: S) {
        let text: String = text.into();
        let (text, text_size) = self.layout_text(&text);
        self.add_text(self.cursor, text);
        self.cursor.y += text_size.y;
        self.cursor.y += self.layout_options.item_spacing.y;
    }

    /// A radio button
    pub fn radio<S: Into<String>>(&mut self, label: S, checked: bool) -> InteractInfo {
        let label: String = label.into();
        let id = self.get_id(&label);
        let (rect, interact) = self.reserve_space(
            id,
            Vec2 {
                x: self.layout_options.width,
                y: self.layout_options.checkbox_radio_height,
            },
        );
        self.commands.push(GuiCmd::RadioButton {
            checked,
            interact,
            rect,
            text: label,
        });
        interact
    }

    pub fn slider_f32<S: Into<String>>(
        &mut self,
        label: S,
        value: &mut f32,
        min: f32,
        max: f32,
    ) -> InteractInfo {
        debug_assert!(min <= max);
        let label: String = label.into();
        let id = self.get_id(&label);
        let (rect, interact) = self.reserve_space(
            id,
            Vec2 {
                x: self.layout_options.width,
                y: self.layout_options.slider_height,
            },
        );

        if interact.active {
            *value = remap_clamp(self.input.mouse_pos.x, rect.min().x, rect.max().x, min, max);
        }

        self.commands.push(GuiCmd::Slider {
            interact,
            label,
            max,
            min,
            rect,
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
        let text: String = text.into();
        let id = self.get_id(&text);
        let (text, text_size) = self.layout_text(&text);
        let text_cursor = self.cursor + self.layout_options.button_padding;
        let (rect, interact) = self.reserve_space(
            id,
            vec2(
                self.layout_options.width,
                text_size.y + 2.0 * self.layout_options.button_padding.y,
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

        self.commands.push(GuiCmd::FoldableHeader {
            interact,
            rect,
            open,
        });
        let icon_width = 16.0; // TODO: this offset is ugly
        self.add_text(text_cursor + vec2(icon_width, 0.0), text);

        if open {
            let old_id = self.id;
            self.id = id;
            let old_x = self.cursor.x;
            self.cursor.x += self.layout_options.indent;
            add_contents(self);
            self.cursor.x = old_x;
            self.id = old_id;
        }

        interact
    }

    // ------------------------------------------------------------------------

    fn reserve_space(&mut self, id: Id, size: Vec2) -> (Rect, InteractInfo) {
        let rect = Rect {
            pos: self.cursor,
            size,
        };
        let interact = self.interactive_rect(id, &rect);
        self.cursor.y += rect.size.y + self.layout_options.item_spacing.y;
        (rect, interact)
    }

    fn interactive_rect(&mut self, id: Id, rect: &Rect) -> InteractInfo {
        let hovered = rect.contains(self.input.mouse_pos);
        let clicked = hovered && self.input.mouse_clicked;
        if clicked {
            self.memory.active_id = Some(id);
        }
        let active = self.memory.active_id == Some(id);

        InteractInfo {
            hovered,
            clicked,
            active,
        }
    }

    fn get_id(&self, id_str: &str) -> Id {
        use std::hash::Hasher;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        hasher.write_u64(self.id);
        hasher.write(id_str.as_bytes());
        hasher.finish()
    }

    fn layout_text(&self, text: &str) -> (TextFragments, Vec2) {
        let char_size = self.layout_options.char_size;
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
            self.commands.push(GuiCmd::Text {
                pos: pos + fragment.rect.pos,
                style: TextStyle::Label,
                text: fragment.text,
                text_align: TextAlign::Start,
            });
        }
    }
}
