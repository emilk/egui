#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::sync::mpsc;
use std::thread::JoinHandle;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
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

struct TestPanel {
    title: String,
    name: String,
    age: u32,
}

impl TestPanel {
    fn new(name: &str, age: u32, thread_nr: usize) -> Self {
        let name = name.into();
        let title = format!("{}'s test panel, thread={}", name, thread_nr);
        Self { title, name, age }
    }

    fn show(&mut self, ctx: &egui::Context) {
        egui::Window::new(&self.title).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Your name: ");
                ui.text_edit_singleline(&mut self.name);
            });
            ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
            if ui.button("Click each year").clicked() {
                self.age += 1;
            }
            ui.label(format!("Hello '{}', age {}", self.name, self.age));
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
            let mut panels = [
                TestPanel::new("Bob", 42 + thread_nr as u32, thread_nr),
                TestPanel::new("Alice", 15 - thread_nr as u32, thread_nr),
                TestPanel::new("Cris", 10 * thread_nr as u32, thread_nr),
            ];

            while let Ok(ctx) = show_rc.recv() {
                for panel in &mut panels {
                    panel.show(&ctx);
                }

                let _ = on_done_tx.send(());
            }
        })
        .expect("failed to spawn thread");
    (handle, show_tx)
}

struct MyApp {
    workers: Vec<(JoinHandle<()>, mpsc::SyncSender<egui::Context>)>,
    on_done_tx: mpsc::SyncSender<()>,
    on_done_rc: mpsc::Receiver<()>,
}

impl MyApp {
    fn new() -> Self {
        let workers = Vec::with_capacity(3);
        let (on_done_tx, on_done_rc) = mpsc::sync_channel(0);

        Self {
            workers,
            on_done_tx,
            on_done_rc,
        }
    }
}

impl std::ops::Drop for MyApp {
    fn drop(&mut self) {
        for (handle, show_tx) in self.workers.drain(..) {
            std::mem::drop(show_tx);
            handle.join().unwrap();
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("add worker").clicked() {
                let thread_nr = self.workers.len();
                self.workers
                    .push(new_worker(thread_nr, self.on_done_tx.clone()));
            }
        });

        for (_handle, show_tx) in &self.workers {
            let _ = show_tx.send(ctx.clone());
        }

        for _ in 0..self.workers.len() {
            let _ = self.on_done_rc.recv();
        }
    }
}
