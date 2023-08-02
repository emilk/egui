use eframe::egui;
use eframe::egui::window::ViewportBuilder;
use eframe::egui::Id;
use eframe::NativeOptions;

fn main() {
    env_logger::init(); // Use `RUST_LOG=debug` to see logs.

    let mut to_repair = false;

    let _ = eframe::run_simple_native(
        "Viewports Examples",
        NativeOptions {
            renderer: eframe::Renderer::Glow,
            ..NativeOptions::default()
        },
        move |ctx, _frame| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label(format!("Frame: {}", ui.ctx().frame_nr()));
                let mut is_desktop = ctx.is_desktop();
                ui.checkbox(&mut is_desktop, "Is Desktop");
                ctx.set_desktop(is_desktop);
                ui.checkbox(&mut to_repair, "To Repair!");

                ctx.create_viewport_sync(
                    ViewportBuilder::default().with_title("Sync rendering!"),
                    |ctx, viewport_id, parent_viewport_id| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Viewport ID: ");
                                ui.label(format!("{viewport_id}"))
                            });
                            ui.horizontal(|ui| {
                                ui.label("Parent Viewport ID: ");
                                ui.label(format!("{parent_viewport_id}"))
                            });
                        })
                    },
                );

                egui::CollapsingHeader::new("Show Test1").show(ui, |ui| {
                    egui::Window::new("Test1").show(ctx, move |ui, id, parent_id| {
                        ui.label(format!("Frame: {}", ui.ctx().frame_nr()));
                        let mut embedded = ui.data_mut(|data| {
                            *data.get_temp_mut_or(Id::new("Test1").with("_embedded"), true)
                        });
                        if ui.checkbox(&mut embedded, "Should embedd?").clicked() {
                            ui.ctx().request_repaint_viewport(parent_id);
                        }
                        ui.data_mut(|data| {
                            data.insert_persisted(Id::new("Test1").with("_embedded"), embedded)
                        });
                        if to_repair {
                            ui.spinner();
                        }

                        let ctx = ui.ctx().clone();
                        ui.label(format!(
                            "Current rendering window: {}",
                            ctx.get_viewport_id()
                        ));
                        if ui.button("Drag").is_pointer_button_down_on() {
                            if id != parent_id {
                                ctx.viewport_command(id, egui::window::ViewportCommand::Drag)
                            } else {
                                ctx.memory_mut(|mem| {
                                    mem.set_dragged_id(egui::Id::new("Test1").with("frame_resize"))
                                });
                            }
                        }
                    });
                });
                egui::CollapsingHeader::new("Shout Test2").show(ui, |ui| {
                    egui::Window::new("Test2").show(ctx, move |ui, id, parent_id| {
                        ui.label(format!("Frame: {}", ui.ctx().frame_nr()));
                        let mut embedded = ui.data_mut(|data| {
                            *data.get_temp_mut_or(Id::new("Test2").with("_embedded"), true)
                        });
                        if ui.checkbox(&mut embedded, "Should embedd?").clicked() {
                            ui.ctx().request_repaint_viewport(parent_id);
                        }
                        ui.data_mut(|data| {
                            data.insert_persisted(Id::new("Test2").with("_embedded"), embedded)
                        });
                        if to_repair {
                            ui.spinner();
                        }
                        let ctx = ui.ctx().clone();
                        ui.label(format!(
                            "Current rendering window: {}",
                            ctx.get_viewport_id()
                        ));

                        if ui.button("Drag").is_pointer_button_down_on() {
                            ctx.viewport_command(id, egui::window::ViewportCommand::Drag)
                        }
                    });
                });
                egui::CollapsingHeader::new("Shout Test3").show(ui, |ui| {
                    egui::Window::new("Test3").show(ctx, move |ui, id, parent_id| {
                        ui.label(format!("Frame: {}", ui.ctx().frame_nr()));
                        let mut embedded = ui.data_mut(|data| {
                            *data.get_temp_mut_or(Id::new("Test3").with("_embedded"), true)
                        });
                        if ui.checkbox(&mut embedded, "Should embedd?").clicked() {
                            ui.ctx().request_repaint_viewport(parent_id);
                        }
                        ui.data_mut(|data| {
                            data.insert_persisted(Id::new("Test3").with("_embedded"), embedded)
                        });
                        let ctx = ui.ctx().clone();
                        ui.label(format!(
                            "Current rendering window: {}",
                            ctx.get_viewport_id()
                        ));

                        if ui.button("Drag").is_pointer_button_down_on() {
                            ctx.viewport_command(id, egui::window::ViewportCommand::Drag)
                        }
                    });
                });
            });
        },
    );
}
