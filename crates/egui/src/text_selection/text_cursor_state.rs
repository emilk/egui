//! Text cursor changes/interaction, without modifying the text.

use emath::Vec2;
use epaint::text::{
    cursor::{ByteCursor, Selection},
    Galley,
};

use crate::{epaint, NumExt as _, Rect, Response, Ui};

/// The state of a text cursor selection.
///
/// Used for [`crate::TextEdit`] and [`crate::Label`].
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TextCursorState {
    selection: Option<Selection>,
}

impl From<Selection> for TextCursorState {
    fn from(selection: Selection) -> Self {
        Self {
            selection: Some(selection),
        }
    }
}

impl TextCursorState {
    pub fn is_empty(&self) -> bool {
        self.selection.is_none()
    }

    pub fn selection(&self) -> Option<Selection> {
        self.selection
    }

    pub fn set_selection(&mut self, selection: Option<Selection>) {
        self.selection = selection;
    }
}

impl TextCursorState {
    /// Handle clicking and/or dragging text.
    ///
    /// Returns `true` if there was interaction.
    pub fn pointer_interaction(
        &mut self,
        ui: &Ui,
        response: &Response,
        pointer_pos: Vec2,
        galley: &Galley,
        is_being_dragged: bool,
    ) -> bool {
        if response.double_clicked() {
            // Select word:
            let selection = galley.selection(|s| s.select_word_at(pointer_pos));
            self.set_selection(Some(selection));
            true
        } else if response.triple_clicked() {
            // Select line:
            let selection = galley.selection(|s| s.select_line_at(pointer_pos));
            self.set_selection(Some(selection));
            true
        } else if response.sense.senses_drag() {
            if response.hovered() && ui.input(|i| i.pointer.any_pressed()) {
                // The start of a drag (or a click).
                if ui.input(|i| i.modifiers.shift) {
                    if let Some(selection) = self.selection() {
                        self.set_selection(Some(
                            galley.selection(|s| {
                                s.extend_selection_to_point(&selection, pointer_pos)
                            }),
                        ));
                    } else {
                        self.set_selection(Some(
                            galley.selection(|s| s.select_single_point_at(pointer_pos)),
                        ));
                    }
                } else {
                    self.set_selection(Some(
                        galley.selection(|s| s.select_single_point_at(pointer_pos)),
                    ));
                }
                true
            } else if is_being_dragged {
                // Drag to select text:
                if let Some(selection) = self.selection() {
                    self.set_selection(Some(
                        galley.selection(|s| s.extend_selection_to_point(&selection, pointer_pos)),
                    ));
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

/// The thin rectangle of one end of the selection, e.g. the primary cursor, in local galley coordinates.
pub fn cursor_rect(galley: &Galley, cursor: &ByteCursor, row_height: f32) -> Rect {
    let mut cursor_pos = galley.pos_from_cursor(*cursor);

    // Handle completely empty galleys
    if cursor_pos.height() < 1.0 {
        cursor_pos.max.y = cursor_pos.max.y.at_least(cursor_pos.min.y + row_height);
    }

    cursor_pos = cursor_pos.expand(1.5); // slightly above/below row

    cursor_pos
}
