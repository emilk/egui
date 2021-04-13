use egui::Id;
use std::hash::Hash;

pub fn view_edit_ui(ui: &mut egui::Ui, text: &mut String, id_source: impl Hash) -> egui::Response {
    #[derive(Clone, Copy, Eq, PartialEq, Debug)]
    #[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
    enum State {
        View,
        Edit,
    }

    impl Default for State {
        fn default() -> Self {
            State::View
        }
    }

    ui.vertical(|ui| {
        let id = Id::new(id_source);

        let mut state = *ui.memory().id_data.get_or_default::<State>(id);

        ui.horizontal(|ui| {
            ui.selectable_value(&mut state, State::View, "View");
            ui.selectable_value(&mut state, State::Edit, "Edit");
        });

        ui.memory().id_data.insert(id, state);

        match state {
            State::View => {
                ui.label(&*text);
            }
            State::Edit => {
                ui.add(
                    egui::TextEdit::multiline(text)
                        .hint_text("Try change this text and enable `View`"),
                );
            }
        }
    })
    .response
}

pub fn widget(text: &mut String, id_source: impl Hash) -> impl egui::Widget + '_ {
    let id = Id::new(id_source);
    move |ui: &mut egui::Ui| view_edit_ui(ui, text, id)
}

pub fn url_to_file_source_code() -> String {
    format!("https://github.com/emilk/egui/blob/master/{}", file!())
}
