use egui::*;

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[derive(PartialEq)]
pub struct MarkdownEditor {
    markdown: String,
}

impl Default for MarkdownEditor {
    fn default() -> Self {
        Self {
            markdown: r#"
# Markdown editor
Markdown support in egui is experimental, and *very* limited. There are:

* bullet points
  * with sub-points
* `inline code`
* *emphasis*
* [hyperlinks](https://github.com/emilk/egui)

---

Also the separator

                "#
            .trim_start()
            .to_owned(),
        }
    }
}

impl super::Demo for MarkdownEditor {
    fn name(&self) -> &str {
        "🖹 Markdown Editor"
    }

    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool) {
        egui::Window::new(self.name()).open(open).show(ctx, |ui| {
            use super::View;
            self.ui(ui);
        });
    }
}

impl super::View for MarkdownEditor {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            egui::reset_button(ui, self);
            ui.add(crate::__egui_github_link_file!());
        });
        ui.separator();
        ui.columns(2, |columns| {
            ScrollArea::auto_sized()
                .id_source("source")
                .show(&mut columns[0], |ui| {
                    ui.text_edit_multiline(&mut self.markdown);
                });
            ScrollArea::auto_sized()
                .id_source("rendered")
                .show(&mut columns[1], |ui| {
                    egui::experimental::markdown(ui, &self.markdown);
                });
        });
    }
}
