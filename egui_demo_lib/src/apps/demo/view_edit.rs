//! Source code example about creating other type of your widget which uses `egui::Memory` and
//! created using a combination of existing widgets.
//! This is meant to be read as a tutorial, hence the plethora of comments.

use egui::Id;
use std::hash::Hash;

/// Easymarkup text editor with the ability to preview a result.
///
/// ## Example:
/// ``` ignore
/// toggle_ui(ui, &mut my_text, "description_1");
/// ```
pub fn view_edit_ui(ui: &mut egui::Ui, text: &mut String, id_source: impl Hash) -> egui::Response {
    // This widget has its own state - `View` or `Edit`,
    // so there is the algorithm for type of widgets:
    //  1. Declare state struct
    //  2. Create id
    //  3. Get state for this widget
    //  4. Process ui, change a local copy of the state
    //  5. Insert changed state back

    // 1. Declare state struct
    // This struct represents the state of this widget. It must implement at least `Clone` and be
    // `'static`. If you use the `persistence` feature, it also must implement
    // `serde::{Deserialize, Serialize}`. You should prefer creating custom newtype structs
    // or enums like this, to avoid TypeId intersection errors, especially when you use
    // `Memory::data` without `Id`.
    #[derive(Clone, Copy, Eq, PartialEq, Debug)]
    #[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
    enum State {
        View,
        Edit,
    }

    // The default state will be set for first call of widget with this id.
    impl Default for State {
        fn default() -> Self {
            State::View
        }
    }

    // 2. Create id
    let id = Id::new(id_source);

    // 3. Get state for this widget
    // You can read more about available `Memory` functions in the documentation of `egui::Memory` struct and `egui::any` module. You should get state by value, not by
    // reference to avoid borrowing of `Memory`.
    let mut state = *ui.memory().id_data.get_or_default::<State>(id);

    // 4. Process ui, change a local copy of the state
    // Sometimes caller could overwrite the default direction, so you must manually specify your
    // preferred direction.
    let result = ui.vertical(|ui| {
        // Here a local copy of the state can be changed by a user.
        ui.horizontal(|ui| {
            ui.selectable_value(&mut state, State::View, "View");
            ui.selectable_value(&mut state, State::Edit, "Edit");
        });

        // Here we use this local state.
        match state {
            State::View => {
                egui::experimental::easy_mark(ui, &*text);
            }
            State::Edit => {
                ui.add(
                    egui::TextEdit::multiline(text)
                        .hint_text("Try change this text and enable `View`"),
                );
            }
        }
    });

    // 5. Insert changed state back
    ui.memory().id_data.insert(id, state);

    // All done! Return the interaction response so the user can check what happened
    // (hovered, clicked, ...) and maybe show a tooltip:
    result.response
}

// A wrapper that allows the more idiomatic usage pattern: `ui.add(toggle(&mut my_bool))`
/// Easymarkup text editor with the ability to preview a result.
///
/// ## Example:
/// ``` ignore
/// ui.add(view_edit(&mut my_text, "description_1"));
/// ```
pub fn view_edit<'a>(text: &'a mut String, id_source: impl Hash + 'a) -> impl egui::Widget + 'a {
    move |ui: &mut egui::Ui| view_edit_ui(ui, text, id_source)
}

pub fn url_to_file_source_code() -> String {
    format!("https://github.com/emilk/egui/blob/master/{}", file!())
}
