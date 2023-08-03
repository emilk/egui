use eframe::egui;
use eframe::egui::window::ViewportBuilder;
use eframe::egui::Id;
use eframe::NativeOptions;

fn main() {
    env_logger::init(); // Use `RUST_LOG=debug` to see logs.

    let mut to_repair = false;

    let mut show_sync = false;
    let mut show = false;
    let mut value = 0.0;

    let mut embedded1 = false;
    let mut embedded2 = true;
    let mut embedded3 = true;

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

                ui.checkbox(&mut show_sync, "Show Sync Viewport");
                if show_sync {
                    ctx.create_viewport_sync(
                        ViewportBuilder::default().with_title("Sync rendering!"),
                        |ctx, viewport_id, parent_viewport_id| {
                            egui::CentralPanel::default().show(ctx, |ui| {
                                ui.label(format!("Frame: {}", ui.ctx().frame_nr()));
                                ui.horizontal(|ui| {
                                    ui.label("Viewport ID: ");
                                    ui.label(format!("{viewport_id}"))
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Parent Viewport ID: ");
                                    ui.label(format!("{parent_viewport_id}"))
                                });
                                ui.checkbox(&mut show, "Show");
                                ui.add(
                                    egui::widgets::DragValue::new(&mut value)
                                        .clamp_range(-10..=10)
                                        .speed(0.1),
                                );
                            })
                        },
                    );
                };

                if show {
                    ui.label("Is shown!");
                    ui.label(format!("Value: {value}"));
                }

                egui::CollapsingHeader::new("Show Test1").show(ui, |ui| {
                    egui::Window::new("Test1")
                        .embedded(&mut embedded1)
                        .show(ctx, |ui| {
                            ui.label(format!("Frame: {}", ui.ctx().frame_nr()));
                            let mut embedded = ui.data_mut(|data| {
                                *data.get_temp_mut_or(Id::new("Test1").with("_is_embedded"), true)
                            });
                            if ui.checkbox(&mut embedded, "Should embedd?").clicked() {
                                ui.ctx()
                                    .request_repaint_viewport(ui.ctx().get_parent_viewport_id());
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
                                if ctx.get_viewport_id() != ctx.get_parent_viewport_id() {
                                    ctx.viewport_command(
                                        ctx.get_viewport_id(),
                                        egui::window::ViewportCommand::Drag,
                                    )
                                } else {
                                    ctx.memory_mut(|mem| {
                                        mem.set_dragged_id(
                                            egui::Id::new("Test1").with("frame_resize"),
                                        )
                                    });
                                }
                            }
                        });
                });
                egui::CollapsingHeader::new("Async Test2").show(ui, |ui| {
                    egui::Window::new("Test2")
                        .embedded(&mut embedded2)
                        .show_async(ctx, move |ui| {
                            ui.label(format!("Frame: {}", ui.ctx().frame_nr()));
                            let mut embedded = ui.data_mut(|data| {
                                *data.get_temp_mut_or(Id::new("Test2").with("_is_embedded"), true)
                            });
                            if ui.checkbox(&mut embedded, "Should embedd?").clicked() {
                                ui.ctx()
                                    .request_repaint_viewport(ui.ctx().get_parent_viewport_id());
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
                                ctx.viewport_command(
                                    ctx.get_viewport_id(),
                                    egui::window::ViewportCommand::Drag,
                                )
                            }
                        });
                });
                egui::CollapsingHeader::new("Async Test3").show(ui, |ui| {
                    egui::Window::new("Test3")
                        .embedded(&mut embedded3)
                        .show_async(ctx, move |ui| {
                            ui.label(format!("Frame: {}", ui.ctx().frame_nr()));
                            let mut embedded = ui.data_mut(|data| {
                                *data.get_temp_mut_or(Id::new("Test3").with("_is_embedded"), true)
                            });
                            if ui.checkbox(&mut embedded, "Should embedd?").clicked() {
                                ui.ctx()
                                    .request_repaint_viewport(ui.ctx().get_parent_viewport_id());
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
                                ctx.viewport_command(
                                    ctx.get_viewport_id(),
                                    egui::window::ViewportCommand::Drag,
                                )
                            }
                        });
                });
            });
        },
    );
}
