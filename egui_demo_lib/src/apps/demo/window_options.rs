#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct WindowOptions {
    title: String,
    title_bar: bool,
    closable: bool,
    collapsible: bool,
    resizable: bool,
    scroll: bool,
    disabled_time: f64,
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
            disabled_time: f64::NEG_INFINITY,
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
            disabled_time,
        } = self.clone();

        let enabled = ctx.input().time - disabled_time > 2.0;
        if !enabled {
            ctx.request_repaint();
        }

        use super::View;
        let mut window = egui::Window::new(title)
            .id(egui::Id::new("demo_window_options")) // required since we change the title
            .resizable(resizable)
            .collapsible(collapsible)
            .title_bar(title_bar)
            .scroll(scroll)
            .enabled(enabled);
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
            disabled_time,
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
        ui.vertical_centered(|ui| {
            ui.add(crate::__egui_github_link_file!());
        });

        if ui.button("Disable for 2 seconds").clicked() {
            *disabled_time = ui.input().time;
        }
    }
}
