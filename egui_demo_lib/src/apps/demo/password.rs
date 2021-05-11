//! Source code example about creating a widget which uses `egui::Memory` to store UI state.
//!
//! This is meant to be read as a tutorial, hence the plethora of comments.

/// Password entry field with ability to toggle character hiding.
///
/// ## Example:
/// ``` ignore
/// password_ui(ui, &mut password);
/// ```
pub fn password_ui(ui: &mut egui::Ui, text: &mut String) -> egui::Response {
    // This widget has its own state — show or hide password characters.

    // 1. Declare state struct
    // This struct represents the state of this widget.
    // It must implement at least `Clone` and be `'static`.
    // If you use the `persistence` feature, it also must implement `serde::{Deserialize, Serialize}`.
    // You should prefer creating custom newtype structs or enums like this, to avoid `TypeId`
    // intersection errors, especially when you use `Memory::data` without `Id`.
    #[derive(Clone, Copy, Default)]
    struct State(bool);

    // 2. Create id
    let id = ui.id().with("show_password");

    // 3. Get state for this widget
    // You can read more about available `Memory` functions in the documentation of `egui::Memory`
    // struct and `egui::any` module.
    // You should get state by value, not by reference to avoid borrowing of `Memory`.
    let mut plaintext = *ui.memory().id_data_temp.get_or_default::<State>(id);

    // 4. Process ui, change a local copy of the state
    // We want TextEdit to fill entire space, and have button after that, so in that case we can
    // change direction to right_to_left.
    let result = ui.with_layout(egui::Layout::right_to_left(), |ui| {
        // Here a local copy of the state can be changed by a user.
        let response = ui
            .add(egui::SelectableLabel::new(plaintext.0, "👁"))
            .on_hover_text("Show/hide password");
        if response.clicked() {
            plaintext.0 = !plaintext.0;
        }

        let text_edit_size = ui.available_size();

        // Here we use this local state.
        ui.add_sized(
            text_edit_size,
            egui::TextEdit::singleline(text).password(!plaintext.0),
        );
    });

    // 5. Insert changed state back
    ui.memory().id_data_temp.insert(id, plaintext);

    // All done! Return the interaction response so the user can check what happened
    // (hovered, clicked, ...) and maybe show a tooltip:
    result.response
}

// A wrapper that allows the more idiomatic usage pattern: `ui.add(...)`
/// Password entry field with ability to toggle character hiding.
///
/// ## Example:
/// ``` ignore
/// ui.add(password(&mut password));
/// ```
pub fn password(text: &mut String) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| password_ui(ui, text)
}

pub fn url_to_file_source_code() -> String {
    format!("https://github.com/emilk/egui/blob/master/{}", file!())
}
