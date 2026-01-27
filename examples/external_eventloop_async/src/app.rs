#![expect(clippy::unwrap_used)] // It's an example

use std::{cell::Cell, io, os::fd::AsRawFd as _, rc::Rc, time::Duration};

use tokio::task::LocalSet;
use winit::event_loop::{ControlFlow, EventLoop};

use eframe::{EframePumpStatus, UserEvent, egui};

pub fn run() -> io::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    let mut eventloop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
    eventloop.set_control_flow(ControlFlow::Poll);

    let mut winit_app = eframe::create_native(
        "External Eventloop Application",
        options,
        Box::new(|_| Ok(Box::<MyApp>::default())),
        &eventloop,
    );

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let local = LocalSet::new();
    local.block_on(&rt, async {
        let eventloop_fd = tokio::io::unix::AsyncFd::new(eventloop.as_raw_fd())?;
        let mut control_flow = ControlFlow::Poll;

        loop {
            let mut guard = match control_flow {
                ControlFlow::Poll => None,
                ControlFlow::Wait => Some(eventloop_fd.readable().await?),
                ControlFlow::WaitUntil(deadline) => {
                    tokio::time::timeout_at(deadline.into(), eventloop_fd.readable())
                        .await
                        .ok()
                        .transpose()?
                }
            };

            match winit_app.pump_eframe_app(&mut eventloop, None) {
                EframePumpStatus::Continue(next) => control_flow = next,
                EframePumpStatus::Exit(code) => {
                    log::info!("exit code: {code}");
                    break;
                }
            }

            if let Some(mut guard) = guard.take() {
                guard.clear_ready();
            }
        }

        Ok::<_, io::Error>(())
    })
}

struct MyApp {
    value: Rc<Cell<u32>>,
    spin: bool,
    blinky: bool,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            value: Rc::new(Cell::new(42)),
            spin: false,
            blinky: false,
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("My External Eventloop Application");

            ui.horizontal(|ui| {
                if ui.button("Increment Now").clicked() {
                    self.value.set(self.value.get() + 1);
                }
                if ui.button("Increment Later").clicked() {
                    let value = Rc::clone(&self.value);
                    let ctx = ui.ctx().clone();
                    tokio::task::spawn_local(async move {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        value.set(value.get() + 1);
                        ctx.request_repaint();
                    });
                }
            });
            ui.label(format!("Value: {}", self.value.get()));

            if ui.button("Toggle Spinner").clicked() {
                self.spin = !self.spin;
            }

            if ui.button("Toggle Blinky").clicked() {
                self.blinky = !self.blinky;
            }

            if self.spin {
                ui.spinner();
            }

            if self.blinky {
                let now = ui.input(|i| i.time);
                let blink = now % 1.0 < 0.5;
                egui::Frame::new()
                    .inner_margin(3)
                    .corner_radius(5)
                    .fill(if blink {
                        egui::Color32::RED
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .show(ui, |ui| {
                        ui.label("Blinky!");
                    });

                ui.request_repaint_after_secs((0.5 - (now % 0.5)) as f32);
            }
        });
    }
}
