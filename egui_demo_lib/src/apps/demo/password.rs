//! Source code example about creating other type of your widget which uses `egui::Memory` and
//! created using a combination of existing widgets.
//! This is meant to be read as a tutorial, hence the plethora of comments.

use egui::Layout;
use std::fmt::Debug;
use std::hash::Hash;

/// Password entry field with ability to toggle character hiding.
///
/// ## Example:
/// ``` ignore
/// password_ui(ui, &mut password, "password_1");
/// ```
pub fn password_ui(
    ui: &mut egui::Ui,
    text: &mut String,
    id_source: impl Hash + Debug,
) -> egui::Response {
    // This widget has its own state — enabled or disabled,
    // so there is the algorithm for this type of widgets:
    //  1. Declare state struct
    //  2. Create id
    //  3. Get state for this widget
    //  4. Process ui, change a local copy of the state
    //  5. Insert changed state back

    // 1. Declare state struct
    // This struct represents the state of this widget.
    // It must implement at least `Clone` and be `'static`. If you use the `persistence` feature,
    // it also must implement `serde::{Deserialize, Serialize}`.
    // You should prefer creating custom newtype structs or enums like this, to avoid TypeId
    // intersection errors, especially when you use `Memory::data` without `Id`.
    #[derive(Clone, Copy, Default)]
    #[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
    struct State(bool);

    // 2. Create id
    let id = ui.make_persistent_id(id_source);

    // 3. Get state for this widget
    // You can read more about available `Memory` functions in the documentation of `egui::Memory`
    // struct and `egui::any` module.
    // You should get state by value, not by reference to avoid borrowing of `Memory`.
    let mut state = *ui.memory().id_data.get_or_default::<State>(id);

    // 4. Process ui, change a local copy of the state
    // We want TextEdit to fill entire space, and have button after that, so in that case we can
    // change direction to right_to_left.
    let result = ui.with_layout(Layout::right_to_left(), |ui| {
        // Here a local copy of the state can be changed by a user.
        let response = ui
            .add(egui::SelectableLabel::new(state.0, "👁"))
            .on_hover_text("Toggle symbols hiding");
        if response.clicked() {
            state.0 = !state.0;
        }

        // Here we use this local state.
        ui.add(egui::TextEdit::singleline(text).password(!state.0));
    });

    // 5. Insert changed state back
    ui.memory().id_data.insert(id, state);

    // All done! Return the interaction response so the user can check what happened
    // (hovered, clicked, ...) and maybe show a tooltip:
    result.response
}

/// Here is the same code again, but a bit more compact:
#[allow(dead_code)]
fn password_ui_compact(
    ui: &mut egui::Ui,
    text: &mut String,
    id_source: impl Hash + Debug,
) -> egui::Response {
    #[derive(Clone, Copy, Default)]
    #[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
    struct State(bool);

    let id = ui.make_persistent_id(id_source);
    let mut state = *ui.memory().id_data.get_or_default::<State>(id);

    let result = ui.with_layout(Layout::right_to_left(), |ui| {
        let response = ui
            .add(egui::SelectableLabel::new(state.0, "👁"))
            .on_hover_text("Toggle symbols hiding");
        if response.clicked() {
            state.0 = !state.0;
        }

        ui.add(egui::TextEdit::singleline(text).password(!state.0));
    });

    ui.memory().id_data.insert(id, state);
    result.response
}

// A wrapper that allows the more idiomatic usage pattern: `ui.add(...)`
/// Password entry field with ability to toggle character hiding.
///
/// ## Example:
/// ``` ignore
/// ui.add(password(&mut password, "password_1"));
/// ```
pub fn password<'a>(
    text: &'a mut String,
    id_source: impl Hash + Debug + 'a,
) -> impl egui::Widget + 'a {
    move |ui: &mut egui::Ui| password_ui(ui, text, id_source)
}

pub fn url_to_file_source_code() -> String {
    format!("https://github.com/emilk/egui/blob/master/{}", file!())
}
