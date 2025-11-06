#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use eframe::egui::mutex::Mutex;
use eframe::egui::{Context, ViewportId};
use std::sync::atomic::AtomicUsize;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Multiple viewports",
        options,
        Box::new(|cc| Ok(Box::<MyApp>::new(MyApp::new(cc.egui_ctx.clone())))),
    )
}

struct MyApp {
    /// Immediate viewports are show immediately, so passing state to/from them is easy.
    /// The downside is that their painting is linked with the parent viewport:
    /// if either needs repainting, they are both repainted.
    show_immediate_viewport: bool,
    immediate_viewport_id: Option<ViewportId>,
    immediate_viewport_redraw_counter: usize,

    /// Deferred viewports run independent of the parent viewport, which can save
    /// CPU if only some of the viewports require repainting.
    /// However, this requires passing state with `Arc` and locks.
    show_deferred_viewport: bool,
    deferred_viewport_id: Option<ViewportId>,
    deferred_viewport_close_requested: Arc<AtomicBool>,
    deferred_viewport_redraw_counter: Arc<AtomicUsize>,

    /// State and handle for a background thread that refreshes the deferred viewport.
    background_thread_mode: Arc<Mutex<BackgroundThreadMode>>,
    background_thread_handle: Option<thread::JoinHandle<u32>>,

    refresh_after_delay: Option<Duration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BackgroundThreadMode {
    AutoRefresh = 0,
    Pause = 1,
    Stop = 2,
}

impl MyApp {
    pub fn new(context: Context) -> Self {
        let background_thread_mode = Arc::new(Mutex::new(BackgroundThreadMode::AutoRefresh));
        let background_thread_handle = thread::spawn({
            let background_thread_mode = background_thread_mode.clone();
            move || {
                let mut counter = 0_u32;
                loop {
                    match *background_thread_mode.lock() {
                        BackgroundThreadMode::AutoRefresh => {
                            println!("refreshing main viewport, counter: {}", counter);
                            context.request_repaint();
                            //context.request_repaint_of(ViewportId::ROOT);
                            counter = counter.wrapping_add(1);
                        }
                        BackgroundThreadMode::Pause => {
                            // nothing to do
                        }
                        BackgroundThreadMode::Stop => {
                            break;
                        }
                    }
                    thread::sleep(Duration::from_millis(100));
                }

                counter
            }
        });

        Self {
            show_immediate_viewport: false,
            immediate_viewport_id: None,
            immediate_viewport_redraw_counter: 0,
            show_deferred_viewport: false,
            deferred_viewport_id: None,
            deferred_viewport_close_requested: Default::default(),
            deferred_viewport_redraw_counter: Default::default(),
            background_thread_mode,
            background_thread_handle: Some(background_thread_handle),

            refresh_after_delay: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        //
        // state handling
        //

        // handle close request from the deferred viewport
        if self
            .deferred_viewport_close_requested
            .load(Ordering::Relaxed)
        {
            self.show_deferred_viewport = false;
        }

        let mut deferred_viewport_refresh_requested = false;

        //
        // root viewport main ui
        //

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello from the root viewport");

            ui.checkbox(
                &mut self.show_immediate_viewport,
                "Show immediate child viewport",
            );

            ui.checkbox(
                &mut self.show_deferred_viewport,
                "Show deferred child viewport",
            );

            {
                // use a scope to limit the mutex lock
                let mut background_thread_mode_guard = self.background_thread_mode.lock();
                let background_thread_mode = &mut *background_thread_mode_guard;
                egui::ComboBox::from_label("Background thread mode")
                    .selected_text(format!("{:?}", background_thread_mode))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            background_thread_mode,
                            BackgroundThreadMode::AutoRefresh,
                            "AutoRefresh",
                        );
                        ui.selectable_value(
                            background_thread_mode,
                            BackgroundThreadMode::Pause,
                            "Pause",
                        );
                    });
            }

            egui::ComboBox::from_label("Refresh after delay")
                .selected_text({
                    match self.refresh_after_delay {
                        None => "Disabled".to_string(),
                        Some(duration) => format!("{}", duration.as_millis()),
                    }
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(&mut self.refresh_after_delay, None, "Disabled")
                        .clicked()
                    {
                        self.refresh_after_delay = None;
                    }
                    if ui
                        .selectable_value(
                            &mut self.refresh_after_delay,
                            Some(Duration::from_millis(100)),
                            "100ms",
                        )
                        .clicked()
                    {
                        self.refresh_after_delay = Some(Duration::from_millis(100));
                    }
                    if ui
                        .selectable_value(
                            &mut self.refresh_after_delay,
                            Some(Duration::from_millis(500)),
                            "500ms",
                        )
                        .clicked()
                    {
                        self.refresh_after_delay = Some(Duration::from_millis(500));
                    }
                    if ui
                        .selectable_value(
                            &mut self.refresh_after_delay,
                            Some(Duration::from_secs(1)),
                            "1s",
                        )
                        .clicked()
                    {
                        self.refresh_after_delay = Some(Duration::from_secs(1));
                    }
                });

            ui.add_enabled_ui(self.show_deferred_viewport, |ui| {
                deferred_viewport_refresh_requested =
                    ui.button("request refresh of deferred viewport").clicked();
            });

            ui.group(|ui| {
                ui.label("root viewport");
                ui.separator();
                ui.label(format!("root frame number: {}", ctx.cumulative_frame_nr()));
            });

            if let Some(immediate_viewport_id) = self.deferred_viewport_id {
                ui.group(|ui| {
                    ui.label("immediate viewport stats");
                    ui.separator();
                    ui.label(format!(
                        "counter: {}",
                        self.immediate_viewport_redraw_counter
                    ));
                    ui.label(format!(
                        "frame number: {}",
                        ctx.cumulative_frame_nr_for(immediate_viewport_id)
                    ));
                });
            }

            if let Some(immediate_viewport_id) = self.deferred_viewport_id {
                ui.group(|ui| {
                    ui.label("deferred viewport stats");
                    ui.separator();
                    ui.label(format!(
                        "counter: {}",
                        self.deferred_viewport_redraw_counter
                            .load(Ordering::Relaxed)
                    ));
                    // Note: this will cause a debug assertion if the viewport is closed.
                    //        we only store the viewport id in self if the viewport is open at the end of the previous frame.
                    ui.label(format!(
                        "frame number: {}",
                        ctx.cumulative_frame_nr_for(immediate_viewport_id)
                    ));
                });
            }
        });

        //
        // viewports
        //

        if self.show_immediate_viewport {
            let immediate_viewport_id = egui::ViewportId::from_hash_of("immediate_viewport");

            ctx.show_viewport_immediate(
                immediate_viewport_id,
                egui::ViewportBuilder::default()
                    .with_title("Immediate Viewport")
                    .with_inner_size([200.0, 100.0]),
                |ctx, class| {
                    assert!(
                        class == egui::ViewportClass::Immediate,
                        "This egui backend doesn't support multiple viewports"
                    );

                    self.immediate_viewport_redraw_counter += 1;

                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.label("Hello from immediate viewport");
                        ui.label(format!(
                            "Counter: {}",
                            self.immediate_viewport_redraw_counter
                        ));
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        // Tell parent viewport that we should not show next frame:
                        self.show_immediate_viewport = false;
                    }
                },
            );
            self.immediate_viewport_id = Some(immediate_viewport_id);
        } else {
            self.immediate_viewport_id = None;
        }

        if self.show_deferred_viewport {
            let deferred_viewport_id = egui::ViewportId::from_hash_of("deferred_viewport");
            let show_deferred_viewport = self.deferred_viewport_close_requested.clone();
            let counter = self.deferred_viewport_redraw_counter.clone();

            if deferred_viewport_refresh_requested {
                ctx.request_repaint_of(deferred_viewport_id);
            }

            ctx.show_viewport_deferred(
                deferred_viewport_id,
                egui::ViewportBuilder::default()
                    .with_title("Deferred Viewport")
                    .with_inner_size([200.0, 100.0]),
                move |ctx, class| {
                    assert!(
                        class == egui::ViewportClass::Deferred,
                        "This egui backend doesn't support multiple viewports"
                    );

                    let value = counter.fetch_add(1, Ordering::Relaxed);

                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.label("Hello from deferred viewport");
                        ui.label(format!("Counter: {value}"));
                    });
                    if ctx.input(|i| i.viewport().close_requested()) {
                        // Tell parent to close us.
                        show_deferred_viewport.store(false, Ordering::Relaxed);
                    }
                },
            );
            self.deferred_viewport_id = Some(deferred_viewport_id);
        } else {
            self.deferred_viewport_id = None;
        }

        if let Some(duration) = self.refresh_after_delay {
            ctx.request_repaint_after(duration);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        *self.background_thread_mode.lock() = BackgroundThreadMode::Stop;
        let counter = self
            .background_thread_handle
            .take()
            .unwrap()
            .join()
            .unwrap();
        println!("background refreshing thread stopped, counter: {}", counter);
    }
}
