#[derive(Default)]
pub struct ExtraViewport {}

impl crate::Demo for ExtraViewport {
    fn is_enabled(&self, ctx: &egui::Context) -> bool {
        !ctx.embed_viewports()
    }

    fn name(&self) -> &'static str {
        "🗖 Extra Viewport"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        if !*open {
            return;
        }

        let id = egui::Id::new(self.name());

        ui.show_viewport_immediate(
            egui::ViewportId(id),
            egui::ViewportBuilder::default()
                .with_title(self.name())
                .with_inner_size([400.0, 512.0]),
            |ui, class| {
                if class == egui::ViewportClass::EmbeddedWindow {
                    // Not a real viewport
                    ui.label("This egui integration does not support multiple viewports");
                } else {
                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        viewport_content(ui, open);
                    });
                }
            },
        );
    }
}

fn viewport_content(ui: &mut egui::Ui, open: &mut bool) {
    ui.label("egui and eframe supports having multiple native windows like this, which egui calls 'viewports'.");

    ui.label(format!(
        "This viewport has id: {:?}, child of viewport {:?}",
        ui.viewport_id(),
        ui.parent_viewport_id()
    ));

    ui.label("Here you can see all the open viewports:");

    egui::ScrollArea::vertical().show(ui, |ui| {
        let viewports = ui.input(|i| i.raw.viewports.clone());
        let ordered_viewports = viewports
            .iter()
            .map(|(id, viewport)| (*id, viewport.clone()))
            .collect::<egui::OrderedViewportIdMap<_>>();
        for (id, viewport) in ordered_viewports {
            ui.group(|ui| {
                ui.label(format!("viewport {id:?}"));
                ui.push_id(id, |ui| {
                    viewport.ui(ui);
                });
            });
        }
    });

    if ui.input(|i| i.viewport().close_requested()) {
        *open = false;
    }
}
