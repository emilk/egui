use std::sync::Arc;
use std::sync::RwLock;

use eframe::egui;
use eframe::egui::ViewportBuilder;
use eframe::NativeOptions;

#[cfg(feature = "wgpu")]
const RENDERER: eframe::Renderer = eframe::Renderer::Wgpu;
#[cfg(not(feature = "wgpu"))]
const RENDERER: eframe::Renderer = eframe::Renderer::Glow;

#[derive(Default)]
pub struct App {
    show_async_viewport: bool,
    show_sync_viewport: bool,
    show_async_window: bool,
    show_sync_window: bool,

    async_viewport_state: Arc<RwLock<usize>>,
    sync_viewport_state: usize,
    async_window_state: Arc<RwLock<usize>>,
    sync_window_state: usize,
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
            ui.label("Look at the \"Frame: \" will tell you, what viewport is rendering!");
            {
                let mut desktop = ctx.is_desktop();
                ui.checkbox(&mut desktop, "Desktop");
                ctx.set_desktop(desktop)
            }
            ui.checkbox(&mut self.show_async_viewport, "Show Async Viewport");
            ui.checkbox(&mut self.show_sync_viewport, "Show Sync Viewport");
            ui.checkbox(&mut self.show_async_window, "Show Async Window");
            ui.checkbox(&mut self.show_sync_window, "Show Sync Window");

            let ctx = ui.ctx();
            // Showing Async Viewport
            if self.show_async_viewport {
                let state = self.async_viewport_state.clone();
                ctx.create_viewport(
                    ViewportBuilder::default().with_title("Async Viewport"),
                    move |ctx| {
                        let mut state = state.write().unwrap();
                        egui::CentralPanel::default().show(ctx, |ui| {
                            ui.label(format!("Frame: {}", ctx.frame_nr()));
                            ui.label(format!("Current Viewport Id: {}", ctx.get_viewport_id()));
                            ui.label(format!(
                                "Current Parent Viewport Id: {}",
                                ctx.get_viewport_id()
                            ));
                            ui.label(format!("Count: {state}"));
                            if ui.button("Add").clicked() {
                                *state += 1;
                            }
                        });
                    },
                )
            }

            // Showing Sync Viewport
            if self.show_sync_viewport {
                ctx.create_viewport_sync(
                    ViewportBuilder::default().with_title("Sync Viewport"),
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            ui.label(format!("Frame: {}", ctx.frame_nr()));
                            ui.label(format!("Current Viewport Id: {}", ctx.get_viewport_id()));
                            ui.label(format!(
                                "Current Parent Viewport Id: {}",
                                ctx.get_viewport_id()
                            ));
                            ui.label(format!("Count: {}", self.sync_viewport_state));
                            if ui.button("Add").clicked() {
                                self.sync_viewport_state += 1;
                            }
                        });
                    },
                );
            }

            // Showing Async Window
            if self.show_async_window {
                let state = self.async_window_state.clone();
                egui::Window::new("Async Window")
                    .default_embedded(false)
                    .show_async(ctx, move |ui| {
                        let ctx = ui.ctx().clone();
                        let mut state = state.write().unwrap();
                        ui.label(format!("Frame: {}", ctx.frame_nr()));
                        ui.label(format!("Current Viewport Id: {}", ctx.get_viewport_id()));
                        ui.label(format!(
                            "Current Parent Viewport Id: {}",
                            ctx.get_viewport_id()
                        ));
                        ui.label(format!("Count: {state}"));
                        if ui.button("Add").clicked() {
                            *state += 1;
                        }
                    });
            }

            // Showing Sync Window
            if self.show_sync_window {
                egui::Window::new("Sync Window")
                    .default_embedded(false)
                    .show(ctx, |ui| {
                        ui.label(format!("Frame: {}", ui.ctx().frame_nr()));
                        ui.label(format!("Frame: {}", ctx.frame_nr()));
                        ui.label(format!("Current Viewport Id: {}", ctx.get_viewport_id()));
                        ui.label(format!("Count: {}", self.sync_window_state));
                        if ui.button("Add").clicked() {
                            self.sync_window_state += 1;
                        }
                    });
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
            initial_window_size: Some(egui::Vec2::new(400.0, 200.0)),
            ..NativeOptions::default()
        },
        Box::new(|_| Box::new(App::default())),
    );
}
