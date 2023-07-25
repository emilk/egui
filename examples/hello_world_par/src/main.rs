//! This example shows that you can use egui in parallel from multiple threads.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::sync::{mpsc, Arc, RwLock};
use std::thread::JoinHandle;

use eframe::egui::{self, ViewportRender};

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1024.0, 768.0)),
        ..Default::default()
    };
    eframe::run_native(
        "My parallel egui App",
        options,
        Box::new(|_cc| Box::new(MyApp::new())),
    )
}

struct ThreadStateData {
    thread_nr: usize,
    title: String,
    name: String,
    age: u32,
}

/// State per thread.
#[derive(Clone)]
struct ThreadState {
    data: Arc<RwLock<ThreadStateData>>,
}

impl ThreadState {
    fn new(thread_nr: usize) -> Self {
        let title = format!("Background thread {thread_nr}");
        Self {
            data: Arc::new(RwLock::new(ThreadStateData {
                thread_nr,
                title,
                name: "Arthur".into(),
                age: 12 + thread_nr as u32 * 10,
            })),
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        let thread_nr = self.data.read().unwrap().thread_nr;
        let pos = egui::pos2(16.0, 128.0 * (thread_nr as f32 + 1.0));
        let clone = self.clone();
        let title = self.data.read().unwrap().title.clone();
        egui::Window::new(title)
            .default_pos(pos)
            .show(ctx, move |ui, _, _| {
                let data = &mut *clone.data.write().unwrap();
                ui.horizontal(|ui| {
                    ui.label("Your name: ");
                    ui.text_edit_singleline(&mut data.name);
                });
                ui.add(egui::Slider::new(&mut data.age, 0..=120).text("age"));
                if ui.button("Click each year").clicked() {
                    data.age += 1;
                }
                ui.label(format!("Hello '{}', age {}", data.name, data.age));
            });
    }
}

fn new_worker(
    thread_nr: usize,
    on_done_tx: mpsc::SyncSender<()>,
) -> (JoinHandle<()>, mpsc::SyncSender<egui::Context>) {
    let (show_tx, show_rc) = mpsc::sync_channel(0);
    let handle = std::thread::Builder::new()
        .name(format!("EguiPanelWorker {}", thread_nr))
        .spawn(move || {
            let mut state = ThreadState::new(thread_nr);
            while let Ok(ctx) = show_rc.recv() {
                state.show(&ctx);
                let _ = on_done_tx.send(());
            }
        })
        .expect("failed to spawn thread");
    (handle, show_tx)
}
struct MyAppData {
    threads: Vec<(JoinHandle<()>, mpsc::SyncSender<egui::Context>)>,
    on_done_tx: mpsc::SyncSender<()>,
}

struct MyApp {
    data: Arc<RwLock<MyAppData>>,
    on_done_rc: mpsc::Receiver<()>,
}

impl MyApp {
    fn new() -> Self {
        let threads = Vec::with_capacity(3);
        let (on_done_tx, on_done_rc) = mpsc::sync_channel(0);

        let mut slf = Self {
            data: Arc::new(RwLock::new(MyAppData {
                threads,
                on_done_tx,
            })),
            on_done_rc,
        };

        {
            let mut data = slf.data.write().unwrap();
            data.spawn_thread();
            data.spawn_thread();
        }

        slf
    }
}

impl MyAppData {
    fn spawn_thread(&mut self) {
        let thread_nr = self.threads.len();
        self.threads
            .push(new_worker(thread_nr, self.on_done_tx.clone()));
    }
}

impl std::ops::Drop for MyApp {
    fn drop(&mut self) {
        for (handle, show_tx) in self.data.write().unwrap().threads.drain(..) {
            std::mem::drop(show_tx);
            handle.join().unwrap();
        }
    }
}

impl eframe::App for MyApp {
    fn update(
        &mut self,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        render: Option<&ViewportRender>,
    ) {
        if let Some(render) = render {
            render(ctx, frame.viewport_id(), frame.parent_viewport_id());
            return;
        }
        let data = self.data.clone();
        egui::Window::new("Main thread").show(ctx, move |ui, _, parent_id| {
            if ui.button("Spawn another thread").clicked() {
                data.write().unwrap().spawn_thread();
                ui.ctx().request_repaint_viewport(parent_id);
            }
        });

        let threads_len;
        {
            let data = self.data.read().unwrap();
            threads_len = data.threads.len();

            for (_handle, show_tx) in &data.threads {
                let _ = show_tx.send(ctx.clone());
            }
        }

        for _ in 0..threads_len {
            let _ = self.on_done_rc.recv();
        }
    }
}
