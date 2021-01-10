use crate::__egui_github_link_file;

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct WindowOptions {
    title: String,
    title_bar: bool,
    closable: bool,
    collapsible: bool,
    resizable: bool,
    scroll: bool,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: "🗖 Window Options".to_owned(),
            title_bar: true,
            closable: true,
            collapsible: true,
            resizable: true,
            scroll: false,
        }
    }
}

impl super::Demo for WindowOptions {
    fn name(&self) -> &str {
        // "🗖 Window Options"
        &self.title
    }

    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool) {
        let Self {
            title,
            title_bar,
            closable,
            collapsible,
            resizable,
            scroll,
        } = self.clone();

        use super::View;
        let mut window = egui::Window::new(title)
            .id(egui::Id::new("demo_window_options")) // required since we change the title
            .resizable(resizable)
            .collapsible(collapsible)
            .title_bar(title_bar)
            .scroll(scroll);
        if closable {
            window = window.open(open);
        }
        window.show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for WindowOptions {
    fn ui(&mut self, ui: &mut egui::Ui) {
        egui::reset_button(ui, self);

        let Self {
            title,
            title_bar,
            closable,
            collapsible,
            resizable,
            scroll,
        } = self;

        ui.horizontal(|ui| {
            ui.label("title:");
            ui.text_edit_singleline(title);
        });
        ui.checkbox(title_bar, "title_bar");
        ui.checkbox(closable, "closable");
        ui.checkbox(collapsible, "collapsible");
        ui.checkbox(resizable, "resizable");
        ui.checkbox(scroll, "scroll");
        ui.add(__egui_github_link_file!());
    }
}
