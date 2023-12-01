#[derive(Default)]
pub struct ExtraViewport {}

impl super::Demo for ExtraViewport {
    fn is_enabled(&self, ctx: &egui::Context) -> bool {
        !ctx.embed_viewports()
    }

    fn name(&self) -> &'static str {
        "🗖 Extra Viewport"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !*open {
            return;
        }

        let id = egui::Id::new(self.name());

        ctx.show_viewport_immediate(
            egui::ViewportId(id),
            egui::ViewportBuilder::default()
                .with_title(self.name())
                .with_inner_size([400.0, 512.0]),
            |ctx, class| {
                if class == egui::ViewportClass::Embedded {
                    // Not a real viewport
                    egui::Window::new(self.name())
                        .id(id)
                        .open(open)
                        .show(ctx, |ui| {
                            ui.label("This egui integration does not support multiple viewports");
                        });
                } else {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        viewport_content(ui, ctx, open);
                    });
                }
            },
        );
    }
}

fn viewport_content(ui: &mut egui::Ui, ctx: &egui::Context, open: &mut bool) {
    ui.label("egui and eframe supports having multiple native windows like this, which egui calls 'viewports'.");

    ui.label(format!(
        "This viewport has id: {:?}, child of viewport {:?}",
        ctx.viewport_id(),
        ctx.parent_viewport_id()
    ));

    ui.label("Here you can see all the open viewports:");

    egui::ScrollArea::vertical().show(ui, |ui| {
        let viewports = ui.input(|i| i.raw.viewports.clone());
        for (id, viewport) in viewports {
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
