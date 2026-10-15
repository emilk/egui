//! Repro: inspecting an app whose window is hidden (minimized or occluded).
//!
//! The app serves the `egui_inspection` protocol and, in the same process, runs an inspector
//! client that asks for a screenshot and for the widget tree once a second. Every request is
//! logged to stderr and in the ui, together with the window state that egui reports.
//!
//! The client drives the whole repro by itself: after a few requests it presses `M` in the app,
//! which minimizes the window for [`HIDE_SECONDS`]. The app restores itself from
//! [`eframe::App::logic`], which keeps ticking while hidden. Covering the window completely
//! with another window (occluding it) is the other half of the repro, and needs a human.

#![allow(clippy::unwrap_used)] // it's an example

use core::time::Duration;
use std::{
    io::{BufReader, BufWriter},
    net::TcpStream,
    sync::mpsc,
    time::Instant,
};

use eframe::egui;
use egui_inspection::{
    InspectionPlugin, Request, Response,
    protocol::{read_handshake, read_message, write_message},
};

/// Where the app listens, and where the client connects.
const ADDR: &str = "127.0.0.1:5719";

/// How long the app stays minimized once the inspector presses `M`.
const HIDE_SECONDS: u64 = 30;

/// One request/response round trip, as shown in the ui.
struct Entry {
    request: String,
    outcome: String,
    seconds: f32,
}

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([700.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "egui_inspection: hidden window",
        options,
        Box::new(|cc| {
            let ctx = &cc.egui_ctx;
            ctx.add_plugin(InspectionPlugin::new(Some(
                "hidden window repro".to_owned(),
            )));
            egui_inspection::serve(ctx, ADDR)?;

            let (tx, rx) = mpsc::channel();
            std::thread::Builder::new()
                .name("inspector".into())
                .spawn(move || run_inspector(&tx))
                .unwrap();

            Ok(Box::new(App {
                log: Vec::new(),
                rx,
                visible: None,
                restore_at: None,
            }))
        }),
    )
}

/// The inspector side: connect, then ask for a screenshot and a tree, over and over.
fn run_inspector(tx: &mpsc::Sender<Entry>) -> ! {
    // Give the listener a moment to come up.
    std::thread::sleep(Duration::from_millis(200));

    let stream = TcpStream::connect(ADDR).unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = BufWriter::new(stream);
    read_handshake(&mut reader).unwrap();

    // A few rounds with the window up, then `M` to minimize it, then the same rounds again
    // while it is hidden.
    let mut round = 0;
    loop {
        round += 1;
        let minimize = round == 3;
        for request in [
            Request::GetScreenshot {
                pixels_per_point: None,
            },
            Request::GetTree,
        ] {
            let name = match request {
                Request::GetScreenshot { .. } => "GetScreenshot",
                _ => "GetTree",
            };
            let start = Instant::now();
            write_message(&mut writer, &request).unwrap();
            let outcome = match read_message::<_, Response>(&mut reader) {
                Ok(Response::Screenshot(png)) => {
                    let [w, h] = png.size;
                    format!("{w}x{h} png, {} kB", png.bytes.len() / 1000)
                }
                Ok(Response::Tree { step, .. }) => format!("tree at step {step}"),
                Ok(Response::Error { message }) => format!("error: {message}"),
                Ok(other) => format!("{other:?}"),
                Err(err) => format!("connection lost: {err}"),
            };
            log::info!(
                "{name} took {:.1} s -> {outcome}",
                start.elapsed().as_secs_f32()
            );
            tx.send(Entry {
                request: name.to_owned(),
                outcome,
                seconds: start.elapsed().as_secs_f32(),
            })
            .ok();
            std::thread::sleep(Duration::from_secs(1));
        }

        if minimize {
            log::info!("pressing M, which minimizes the window for {HIDE_SECONDS} s");
            write_message(&mut writer, &press_m()).unwrap();
            read_message::<_, Response>(&mut reader).ok();
        }
    }
}

/// The `M` keypress that the app turns into a minimize.
fn press_m() -> Request {
    let key = |pressed| egui::Event::Key {
        key: egui::Key::M,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    Request::ApplyEvents {
        events: vec![key(true), key(false)],
    }
}

struct App {
    log: Vec<Entry>,
    rx: mpsc::Receiver<Entry>,
    visible: Option<bool>,

    /// When to restore the window we minimized, so the repro does not need a human to bring
    /// the app back.
    restore_at: Option<Instant>,
}

impl eframe::App for App {
    /// Runs even while the window is hidden, so it can report the window state.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let visible = ctx.input(|i| i.viewport().visible());
        if visible != self.visible {
            self.visible = visible;
            log::info!("window visible: {visible:?}");
        }

        if self.restore_at.is_some_and(|at| at <= Instant::now()) {
            self.restore_at = None;
            log::info!("restoring the window");
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        }
        // `logic` only runs when something asks for it, so keep asking:
        ctx.request_repaint_after(Duration::from_millis(100));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.log.extend(self.rx.try_iter());
        let ctx = ui.ctx().clone();

        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Inspecting a hidden window");
            ui.label(
                "The inspector client minimizes this window by itself after a few requests. \
                 Covering the window completely with another one is the other half of the \
                 repro, and needs a human.",
            );

            ui.add_space(8.0);
            let pressed_m = ui.input(|i| i.key_pressed(egui::Key::M));
            if ui
                .button(format!("Minimize for {HIDE_SECONDS} s (or press M)"))
                .clicked()
                || pressed_m
            {
                self.restore_at = Some(Instant::now() + Duration::from_secs(HIDE_SECONDS));
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }

            ui.add_space(8.0);
            let info = ctx.input(|i| i.viewport().clone());
            ui.label(format!(
                "minimized: {:?}, occluded: {:?}, visible: {:?}",
                info.minimized,
                info.occluded,
                info.visible()
            ));

            // Something that moves, so it is obvious in a screenshot which frame it is:
            ui.add_space(8.0);
            ui.add(egui::Spinner::new());
            ctx.request_repaint();

            ui.add_space(8.0);
            ui.separator();
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for Entry {
                        request,
                        outcome,
                        seconds,
                    } in &self.log
                    {
                        ui.label(format!("{request} took {seconds:.1} s → {outcome}"));
                    }
                });
        });
    }
}
