#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use eframe::egui;
use std::sync::mpsc;
use std::thread::JoinHandle;

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| Box::new(MyApp::new())),
    );
}

struct TestPanel {
    title: String,
    name: String,
    age: u32,
}

impl TestPanel {
    fn show(&mut self, ctx: &egui::Context) {
        egui::Window::new(&self.title).show(ctx, |ui| {
            ui.heading("My egui Application");
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

    fn new(name: &str, age: u32, id: usize) -> Self {
        let name = name.into();
        let title = format!("{}'s test panel {}", name, id);
        Self { title, name, age }
    }
}

fn new_worker(
    id: usize,
    on_done_tx: mpsc::SyncSender<()>,
) -> (JoinHandle<()>, mpsc::SyncSender<egui::Context>) {
    let (show_tx, show_rc) = mpsc::sync_channel(0);
    let handle = std::thread::Builder::new()
        .name(format!("EguiPanelWorker {}", id))
        .spawn(move || {
            let mut panels = [
                TestPanel::new("Bob", 42 + id as u32, id),
                TestPanel::new("Alice", 15 - id as u32, id),
                TestPanel::new("Cris", 10 * id as u32, id),
            ];

            while let Ok(ctx) = show_rc.recv() {
                for panel in &mut panels {
                    panel.show(&ctx)
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
                let id = self.workers.len();
                self.workers.push(new_worker(id, self.on_done_tx.clone()))
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
