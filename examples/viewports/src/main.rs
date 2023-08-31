use std::sync::Arc;
use std::sync::RwLock;

use eframe::egui;
use eframe::egui::ViewportBuilder;
use eframe::egui::ViewportId;
use eframe::NativeOptions;

#[cfg(feature = "wgpu")]
const RENDERER: eframe::Renderer = eframe::Renderer::Wgpu;
#[cfg(not(feature = "wgpu"))]
const RENDERER: eframe::Renderer = eframe::Renderer::Glow;

#[derive(Default)]
pub struct App {
    show_async_viewport: bool,
    show_sync_viewport: bool,

    async_viewport_state: Arc<RwLock<usize>>,
    sync_viewport_state: usize,

    show_async_viewport2: Arc<RwLock<bool>>,
    show_sync_viewport2: Arc<RwLock<bool>>,

    async_viewport_state2: Arc<RwLock<usize>>,
    sync_viewport_state2: Arc<RwLock<usize>>,
}

impl eframe::App for App {
    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
        render_function: Option<&egui::ViewportRender>,
    ) {
        // This needs to be like this to be able to show stuf on a async viewport
        if let Some(render) = render_function {
            // This is the render function for the current async viewport
            render(ctx);
            return;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!("Frame: {}", ctx.frame_nr()));
            ui.label(format!("Current Viewport Id: {}", ctx.get_viewport_id()));
            ui.label(format!(
                "Current Parent Viewport Id: {}",
                ctx.get_viewport_id()
            ));
            ui.label(format!("Pos: {:?}", ctx.viewport_outer_pos()));
            ui.label(format!("Size: {:?}", ctx.viewport_inner_size()));
            ui.label("Look at the \"Frame: \" will tell you, what viewport is rendering!");
            {
                let mut force_embedding = ctx.force_embedding();
                ui.checkbox(&mut force_embedding, "Force embedding!");
                ctx.set_force_embedding(force_embedding);
            }
            ui.checkbox(&mut self.show_async_viewport, "Show Async Viewport");
            ui.checkbox(&mut self.show_sync_viewport, "Show Sync Viewport");

            let ctx = ui.ctx();
            // Showing Async Viewport
            if self.show_async_viewport {
                let state = self.async_viewport_state.clone();

                let show_async_viewport2 = self.show_async_viewport2.clone();
                let show_sync_viewport2 = self.show_sync_viewport2.clone();

                let async_viewport_state2 = self.async_viewport_state2.clone();
                let sync_viewport_state2 = self.sync_viewport_state2.clone();

                ctx.create_viewport(
                    ViewportBuilder::new("Async Viewport").with_title("Async Viewport"),
                    move |ctx| {
                        let mut state = state.write().unwrap();

                        let mut show_async_viewport2 = show_async_viewport2.write().unwrap();
                        let mut show_sync_viewport2 = show_sync_viewport2.write().unwrap();

                        let async_viewport_state2 = async_viewport_state2.clone();
                        let sync_viewport_state2 = sync_viewport_state2.clone();

                        let content = move |ui: &mut egui::Ui| {
                            ui.label(format!("Frame: {}", ctx.frame_nr()));
                            ui.label(format!("Current Viewport Id: {}", ctx.get_viewport_id()));
                            ui.label(format!(
                                "Current Parent Viewport Id: {}",
                                ctx.get_viewport_id()
                            ));
                            ui.label(format!("Pos: {:?}", ctx.viewport_outer_pos()));
                            ui.label(format!("Size: {:?}", ctx.viewport_inner_size()));

                            ui.checkbox(&mut show_async_viewport2, "Show Async Viewport 2");
                            ui.checkbox(&mut show_sync_viewport2, "Show Sync Viewport 2");

                            ui.label(format!("Count: {state}"));
                            if ui.button("Add").clicked() {
                                *state += 1;
                            }

                            if *show_async_viewport2 {
                                ctx.create_viewport(
                                    ViewportBuilder::new("Async Viewport in Async Viewport")
                                        .with_title("Async Viewport in Async Viewport"),
                                    move |ctx| {
                                        let mut state = async_viewport_state2.write().unwrap();

                                        let content = move |ui: &mut egui::Ui| {
                                            ui.label(format!("Frame: {}", ctx.frame_nr()));
                                            ui.label(format!(
                                                "Current Viewport Id: {}",
                                                ctx.get_viewport_id()
                                            ));
                                            ui.label(format!(
                                                "Current Parent Viewport Id: {}",
                                                ctx.get_viewport_id()
                                            ));
                                            ui.label(format!(
                                                "Pos: {:?}",
                                                ctx.viewport_outer_pos()
                                            ));
                                            ui.label(format!(
                                                "Size: {:?}",
                                                ctx.viewport_inner_size()
                                            ));

                                            ui.label(format!("Count: {state}"));
                                            if ui.button("Add").clicked() {
                                                *state += 1;
                                            }
                                        };

                                        // This will make the viewport content, a popup if is in the main window
                                        if ctx.get_viewport_id() == ViewportId::MAIN {
                                            egui::Area::new("Async Viewport in Async Viewport")
                                                .show(ctx, |ui| {
                                                    egui::Frame::popup(ui.style())
                                                        .show(ui, content);
                                                });
                                        } else {
                                            egui::CentralPanel::default().show(ctx, content);
                                        };
                                    },
                                );
                            }

                            if *show_sync_viewport2 {
                                ctx.create_viewport_sync(
                                    ViewportBuilder::new("Sync Viewport in Async Viewport")
                                        .with_title("Sync Viewport in Async Viewport"),
                                    move |ctx| {
                                        let mut state = sync_viewport_state2.write().unwrap();

                                        let content = move |ui: &mut egui::Ui| {
                                            ui.label(format!("Frame: {}", ctx.frame_nr()));
                                            ui.label(format!(
                                                "Current Viewport Id: {}",
                                                ctx.get_viewport_id()
                                            ));
                                            ui.label(format!(
                                                "Current Parent Viewport Id: {}",
                                                ctx.get_viewport_id()
                                            ));
                                            ui.label(format!(
                                                "Pos: {:?}",
                                                ctx.viewport_outer_pos()
                                            ));
                                            ui.label(format!(
                                                "Size: {:?}",
                                                ctx.viewport_inner_size()
                                            ));

                                            ui.label(format!("Count: {state}"));
                                            if ui.button("Add").clicked() {
                                                *state += 1;
                                            }
                                        };

                                        // This will make the viewport content, a popup if is in the main window
                                        if ctx.get_viewport_id() == ViewportId::MAIN {
                                            egui::Area::new("Sync Viewport in Async Viewport")
                                                .show(ctx, |ui| {
                                                    egui::Frame::popup(ui.style())
                                                        .show(ui, content);
                                                });
                                        } else {
                                            egui::CentralPanel::default().show(ctx, content);
                                        };
                                    },
                                );
                            }
                        };

                        // This will make the viewport content, a popup if is in the main window
                        if ctx.get_viewport_id() == ViewportId::MAIN {
                            egui::Area::new("Async Viewport").show(ctx, |ui| {
                                egui::Frame::popup(ui.style()).show(ui, content);
                            });
                        } else {
                            egui::CentralPanel::default().show(ctx, content);
                        }
                    },
                );
            }

            // Showing Sync Viewport
            if self.show_sync_viewport {
                ctx.create_viewport_sync(
                    ViewportBuilder::new("Sync Viewport").with_title("Sync Viewport"),
                    |ctx| {
                        let mut show_async_viewport2 = self.show_async_viewport2.write().unwrap();
                        let mut show_sync_viewport2 = self.show_sync_viewport2.write().unwrap();

                        let async_viewport_state2 = self.async_viewport_state2.clone();
                        let sync_viewport_state2 = self.sync_viewport_state2.clone();

                        let content =
                            |ui: &mut egui::Ui| {
                                ui.label(format!("Frame: {}", ctx.frame_nr()));
                                ui.label(format!("Current Viewport Id: {}", ctx.get_viewport_id()));
                                ui.label(format!(
                                    "Current Parent Viewport Id: {}",
                                    ctx.get_viewport_id()
                                ));
                                ui.label(format!("Pos: {:?}", ctx.viewport_outer_pos()));
                                ui.label(format!("Size: {:?}", ctx.viewport_inner_size()));

                                ui.checkbox(&mut show_async_viewport2, "Show Async Viewport");
                                ui.checkbox(&mut show_sync_viewport2, "Show Sync Viewport");

                                ui.label(format!("Count: {}", self.sync_viewport_state));
                                if ui.button("Add").clicked() {
                                    self.sync_viewport_state += 1;
                                }

                                if *show_async_viewport2 {
                                    ctx.create_viewport(
                                        ViewportBuilder::new("Async Viewport in Sync Viewport")
                                            .with_title("Async Viewport in Sync Viewport"),
                                        move |ctx| {
                                            let mut state = async_viewport_state2.write().unwrap();

                                            let content = move |ui: &mut egui::Ui| {
                                                ui.label(format!("Frame: {}", ctx.frame_nr()));
                                                ui.label(format!(
                                                    "Current Viewport Id: {}",
                                                    ctx.get_viewport_id()
                                                ));
                                                ui.label(format!(
                                                    "Current Parent Viewport Id: {}",
                                                    ctx.get_viewport_id()
                                                ));
                                                ui.label(format!(
                                                    "Pos: {:?}",
                                                    ctx.viewport_outer_pos()
                                                ));
                                                ui.label(format!(
                                                    "Size: {:?}",
                                                    ctx.viewport_inner_size()
                                                ));

                                                ui.label(format!("Count: {state}"));
                                                if ui.button("Add").clicked() {
                                                    *state += 1;
                                                }
                                            };

                                            // This will make the viewport content, a popup if is in the main window
                                            if ctx.get_viewport_id() == ViewportId::MAIN {
                                                egui::Area::new("Async Viewport in Sync Viewport")
                                                    .show(ctx, |ui| {
                                                        egui::Frame::popup(ui.style())
                                                            .show(ui, content);
                                                    });
                                            } else {
                                                egui::CentralPanel::default().show(ctx, content);
                                            };
                                        },
                                    );
                                }

                                if *show_sync_viewport2 {
                                    ctx.create_viewport_sync(
                                        ViewportBuilder::new("Sync Viewport in Sync Viewport")
                                            .with_title("Sync Viewport in Sync Viewport"),
                                        move |ctx| {
                                            let mut state = sync_viewport_state2.write().unwrap();

                                            let content = move |ui: &mut egui::Ui| {
                                                ui.label(format!("Frame: {}", ctx.frame_nr()));
                                                ui.label(format!(
                                                    "Current Viewport Id: {}",
                                                    ctx.get_viewport_id()
                                                ));
                                                ui.label(format!(
                                                    "Current Parent Viewport Id: {}",
                                                    ctx.get_viewport_id()
                                                ));
                                                ui.label(format!(
                                                    "Pos: {:?}",
                                                    ctx.viewport_outer_pos()
                                                ));
                                                ui.label(format!(
                                                    "Size: {:?}",
                                                    ctx.viewport_inner_size()
                                                ));

                                                ui.label(format!("Count: {state}"));
                                                if ui.button("Add").clicked() {
                                                    *state += 1;
                                                }
                                            };

                                            // This will make the viewport content, a popup if is in the main window
                                            if ctx.get_viewport_id() == ViewportId::MAIN {
                                                egui::Area::new("Sync Viewport in Sync Viewport")
                                                    .show(ctx, |ui| {
                                                        egui::Frame::popup(ui.style())
                                                            .show(ui, content);
                                                    });
                                            } else {
                                                egui::CentralPanel::default().show(ctx, content);
                                            };
                                        },
                                    );
                                }
                            };

                        // This will make the viewport content, a popup if is in the main window
                        if ctx.get_viewport_id() == ViewportId::MAIN {
                            egui::Area::new("Sync Viewport").show(ctx, |ui| {
                                egui::Frame::popup(ui.style()).show(ui, content);
                            });
                        } else {
                            egui::CentralPanel::default().show(ctx, content);
                        }
                    },
                );
            }
        });
    }
}

fn main() {
    env_logger::init(); // Use `RUST_LOG=debug` to see logs.

    let _ = eframe::run_native(
        "Viewports",
        NativeOptions {
            renderer: RENDERER,
            initial_window_size: Some(egui::Vec2::new(400.0, 220.0)),
            ..NativeOptions::default()
        },
        Box::new(|_| Box::<App>::default()),
    );
}
