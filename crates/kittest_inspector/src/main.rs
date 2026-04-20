//! Eframe app that displays frames + accesskit trees streamed from an `egui_kittest` harness,
//! and lets the user pause / resume / single-step the test and inspect individual widgets.
//!
//! Communication is over stdin/stdout: the harness pipes [`HarnessMessage`]s into our stdin
//! and reads [`InspectorReply`]s from our stdout. All logging goes to stderr.

#![expect(clippy::print_stderr)] // The inspector binary's only logging channel is stderr.

use std::io::{self, BufReader, BufWriter};
use std::sync::mpsc;
use std::thread;

use eframe::egui;
use kittest_inspector::{
    Frame, HarnessMessage, InspectorReply, read_message, write_message,
};

use accesskit::{Node, NodeId, Rect as AkRect};

/// Internal worker → UI message.
enum WorkerEvent {
    Frame(Frame),
    Disconnected,
}

/// UI → worker message: "you may send `Continue` to the harness now".
type ReleaseTx = mpsc::Sender<()>;
type ReleaseRx = mpsc::Receiver<()>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayState {
    Playing,
    Paused,
}

fn main() -> eframe::Result<()> {
    let (worker_tx, worker_rx) = mpsc::channel::<WorkerEvent>();
    let (release_tx, release_rx) = mpsc::channel::<()>();

    thread::Builder::new()
        .name("kittest_inspector_io".into())
        .spawn(move || run_io(&worker_tx, &release_rx))
        .expect("spawn io thread");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("kittest inspector")
            .with_inner_size([1100.0, 750.0]),
        ..Default::default()
    };

    eframe::run_native(
        "kittest inspector",
        options,
        Box::new(|cc| Ok(Box::new(InspectorApp::new(cc, worker_rx, release_tx)))),
    )
}

/// Read frames from stdin, forward to UI, wait for a release, then write Continue to stdout.
fn run_io(ui_tx: &mpsc::Sender<WorkerEvent>, release_rx: &ReleaseRx) {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());

    loop {
        match read_message::<_, HarnessMessage>(&mut reader) {
            Ok(HarnessMessage::Frame(frame)) => {
                if ui_tx.send(WorkerEvent::Frame(frame)).is_err() {
                    return;
                }
                if release_rx.recv().is_err() {
                    return;
                }
                if let Err(err) = write_message(&mut writer, &InspectorReply::Continue) {
                    eprintln!("kittest_inspector: write failed: {err}");
                    return;
                }
            }
            Ok(HarnessMessage::Goodbye) => {
                let _ = ui_tx.send(WorkerEvent::Disconnected);
                return;
            }
            Err(err) => {
                if err.kind() != io::ErrorKind::UnexpectedEof {
                    eprintln!("kittest_inspector: read failed: {err}");
                }
                let _ = ui_tx.send(WorkerEvent::Disconnected);
                return;
            }
        }
    }
}

struct InspectorApp {
    worker_rx: mpsc::Receiver<WorkerEvent>,
    release_tx: ReleaseTx,
    play_state: PlayState,
    /// True when the worker is blocked waiting for a release.
    worker_waiting: bool,
    current_frame: Option<Frame>,
    current_texture: Option<egui::TextureHandle>,
    received_count: u64,
    connected: bool,
    /// Currently hovered widget (cleared every frame, set during central-panel paint).
    hovered_node: Option<NodeId>,
    /// Last clicked widget (sticky).
    selected_node: Option<NodeId>,
}

impl InspectorApp {
    fn new(
        _cc: &eframe::CreationContext<'_>,
        worker_rx: mpsc::Receiver<WorkerEvent>,
        release_tx: ReleaseTx,
    ) -> Self {
        Self {
            worker_rx,
            release_tx,
            play_state: PlayState::Paused,
            worker_waiting: false,
            current_frame: None,
            current_texture: None,
            received_count: 0,
            connected: true,
            hovered_node: None,
            selected_node: None,
        }
    }

    fn pump_worker(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.worker_rx.try_recv() {
            match event {
                WorkerEvent::Frame(frame) => {
                    self.received_count += 1;
                    self.upload_frame(ctx, &frame);
                    // Keep the selection sticky across frames (same NodeId may still exist).
                    self.current_frame = Some(frame);
                    self.worker_waiting = true;
                    if self.play_state == PlayState::Playing {
                        self.send_release();
                    }
                }
                WorkerEvent::Disconnected => {
                    self.connected = false;
                    self.worker_waiting = false;
                }
            }
        }
    }

    fn upload_frame(&mut self, ctx: &egui::Context, frame: &Frame) {
        let size = [frame.width as usize, frame.height as usize];
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &frame.rgba);
        let texture = ctx.load_texture("kittest_inspector_frame", color_image, Default::default());
        self.current_texture = Some(texture);
    }

    fn send_release(&mut self) {
        if !self.worker_waiting {
            return;
        }
        if self.release_tx.send(()).is_ok() {
            self.worker_waiting = false;
        }
    }
}

impl eframe::App for InspectorApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.pump_worker(&ctx);
        // Reset hover each frame — central panel will set it again if mouse is over the image.
        self.hovered_node = None;

        controls_panel(self, ui);
        details_panel(self, ui);
        central_panel(self, ui);

        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}

fn controls_panel(app: &mut InspectorApp, ui: &mut egui::Ui) {
    egui::Panel::top("controls").show_inside(ui, |ui| {
        ui.horizontal(|ui| {
            let playing = app.play_state == PlayState::Playing;
            if ui.selectable_label(playing, "▶ Play").clicked() {
                app.play_state = PlayState::Playing;
                app.send_release();
            }
            if ui
                .selectable_label(!playing, "⏸ Pause")
                .on_hover_text("Pause harness after the next frame")
                .clicked()
            {
                app.play_state = PlayState::Paused;
            }
            let can_step = app.play_state == PlayState::Paused && app.worker_waiting;
            if ui
                .add_enabled(can_step, egui::Button::new("⏭ Next"))
                .on_hover_text("Advance one harness step")
                .clicked()
            {
                app.send_release();
            }

            ui.separator();
            ui.label(format!(
                "frames: {}  |  state: {:?}  |  {}",
                app.received_count,
                app.play_state,
                if app.connected {
                    if app.worker_waiting {
                        "harness blocked"
                    } else {
                        "harness running"
                    }
                } else {
                    "harness disconnected"
                }
            ));
        });
    });
}

fn details_panel(app: &mut InspectorApp, ui: &mut egui::Ui) {
    egui::Panel::right("details")
        .resizable(true)
        .default_size(380.0)
        .show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let Some(frame) = app.current_frame.clone() else {
                    ui.weak("Waiting for frames...");
                    return;
                };

                egui::CollapsingHeader::new("Frame")
                    .default_open(true)
                    .show(ui, |ui| {
                        kv_grid(ui, "frame_grid", |ui| {
                            if let Some(label) = &frame.label {
                                ui.label("Test:");
                                ui.monospace(label);
                                ui.end_row();
                            }
                            ui.label("Step:");
                            ui.monospace(frame.step.to_string());
                            ui.end_row();
                            ui.label("Size (px):");
                            ui.monospace(format!("{} × {}", frame.width, frame.height));
                            ui.end_row();
                            ui.label("Pixels per point:");
                            ui.monospace(format!("{:.2}", frame.pixels_per_point));
                            ui.end_row();
                            let node_count = frame
                                .accesskit
                                .as_ref()
                                .map_or(0, |u| u.nodes.len());
                            ui.label("AccessKit nodes:");
                            ui.monospace(node_count.to_string());
                            ui.end_row();
                        });
                    });

                let target = app.selected_node.or(app.hovered_node);
                let header = if app.selected_node.is_some() {
                    "Selected widget"
                } else if app.hovered_node.is_some() {
                    "Hovered widget"
                } else {
                    "Widget"
                };
                egui::CollapsingHeader::new(header)
                    .default_open(true)
                    .show(ui, |ui| match (target, &frame.accesskit) {
                        (Some(id), Some(update)) => {
                            if let Some((_, node)) = update.nodes.iter().find(|(nid, _)| *nid == id)
                            {
                                widget_details(ui, id, node);
                            } else {
                                ui.weak("(node not in latest tree)");
                            }
                        }
                        _ => {
                            ui.weak("Hover over the rendered frame to inspect a widget.");
                        }
                    });

                if app.selected_node.is_some()
                    && ui
                        .small_button("clear selection")
                        .on_hover_text("Stop pinning the selected widget")
                        .clicked()
                {
                    app.selected_node = None;
                }

                egui::CollapsingHeader::new("All AccessKit nodes")
                    .default_open(false)
                    .show(ui, |ui| {
                        if let Some(update) = &frame.accesskit {
                            for (id, node) in &update.nodes {
                                let role = format!("{:?}", node.role());
                                let label = node
                                    .label()
                                    .map(str::to_owned)
                                    .or_else(|| node.value().map(str::to_owned))
                                    .unwrap_or_default();
                                let selected = Some(*id) == app.selected_node;
                                if ui
                                    .selectable_label(
                                        selected,
                                        format!("{:?}  {role}  {label:?}", id.0),
                                    )
                                    .clicked()
                                {
                                    app.selected_node = Some(*id);
                                }
                            }
                        } else {
                            ui.weak("(no accesskit tree)");
                        }
                    });
            });
        });
}

fn central_panel(app: &mut InspectorApp, ui: &mut egui::Ui) {
    egui::CentralPanel::default().show_inside(ui, |ui| {
        let Some(tex) = app.current_texture.clone() else {
            ui.centered_and_justified(|ui| {
                ui.label("Waiting for harness to connect...");
            });
            return;
        };
        let Some(frame) = app.current_frame.clone() else {
            return;
        };

        let physical = tex.size_vec2(); // physical pixels of the rendered frame
        let avail = ui.available_size();
        let scale = (avail.x / physical.x).min(avail.y / physical.y).clamp(0.05, 1.0);
        let display_size = physical * scale;

        let (image_rect, response) = ui.allocate_exact_size(
            display_size,
            egui::Sense::click().union(egui::Sense::hover()),
        );
        ui.painter().image(
            tex.id(),
            image_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );

        // logical_point → screen_position:
        //     screen = image_rect.min + ak_rect * pixels_per_point * scale
        let logical_to_screen = |r: AkRect| -> egui::Rect {
            let f = frame.pixels_per_point * scale;
            egui::Rect::from_min_max(
                image_rect.min + egui::vec2(r.x0 as f32 * f, r.y0 as f32 * f),
                image_rect.min + egui::vec2(r.x1 as f32 * f, r.y1 as f32 * f),
            )
        };
        let screen_to_logical = |p: egui::Pos2| -> (f64, f64) {
            let f = (frame.pixels_per_point * scale) as f64;
            (
                ((p.x - image_rect.min.x) as f64) / f,
                ((p.y - image_rect.min.y) as f64) / f,
            )
        };

        // Hit test: smallest containing widget wins.
        if let (Some(pos), Some(update)) = (response.hover_pos(), &frame.accesskit) {
            let (lx, ly) = screen_to_logical(pos);
            let mut best: Option<(NodeId, f64)> = None;
            for (id, node) in &update.nodes {
                let Some(b) = node.bounds() else { continue };
                if lx >= b.x0 && lx <= b.x1 && ly >= b.y0 && ly <= b.y1 {
                    let area = (b.x1 - b.x0).max(0.0) * (b.y1 - b.y0).max(0.0);
                    if best.is_none_or(|(_, a)| area < a) {
                        best = Some((*id, area));
                    }
                }
            }
            app.hovered_node = best.map(|(id, _)| id);
        }
        if response.clicked() {
            app.selected_node = app.hovered_node;
        }

        let painter = ui.painter_at(image_rect);
        if let Some(update) = &frame.accesskit {
            // Highlight selection (blue) and hover (yellow).
            let draw = |id: NodeId, color: egui::Color32| {
                if let Some((_, node)) = update.nodes.iter().find(|(nid, _)| *nid == id)
                    && let Some(b) = node.bounds()
                {
                    painter.rect_stroke(
                        logical_to_screen(b),
                        2.0,
                        egui::Stroke::new(1.5, color),
                        egui::StrokeKind::Outside,
                    );
                }
            };
            if let Some(id) = app.selected_node {
                draw(id, egui::Color32::from_rgb(80, 180, 255));
            }
            if let Some(id) = app.hovered_node
                && app.hovered_node != app.selected_node
            {
                draw(id, egui::Color32::from_rgb(255, 220, 90));
            }
        }
    });
}

fn kv_grid(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id)
        .num_columns(2)
        .striped(true)
        .show(ui, body);
}

/// Render the inspector grid for a single accesskit node, mimicking egui's `inspection_ui`.
fn widget_details(ui: &mut egui::Ui, id: NodeId, node: &Node) {
    kv_grid(ui, "widget_grid", |ui| {
        ui.label("ID:");
        ui.monospace(format!("{:?}", id.0));
        ui.end_row();

        ui.label("Role:");
        ui.monospace(format!("{:?}", node.role()));
        ui.end_row();

        if let Some(b) = node.bounds() {
            ui.label("Bounds:");
            ui.monospace(format!(
                "({:.1}, {:.1}) → ({:.1}, {:.1})  [{:.1} × {:.1}]",
                b.x0,
                b.y0,
                b.x1,
                b.y1,
                b.x1 - b.x0,
                b.y1 - b.y0,
            ));
            ui.end_row();
        }

        for (label, value) in [
            ("Label:", node.label()),
            ("Value:", node.value()),
            ("Description:", node.description()),
            ("Placeholder:", node.placeholder()),
            ("Tooltip:", node.tooltip()),
            ("Class:", node.class_name()),
            ("Author ID:", node.author_id()),
            ("Keyboard:", node.keyboard_shortcut()),
        ] {
            if let Some(v) = value
                && !v.is_empty()
            {
                ui.label(label);
                ui.monospace(v);
                ui.end_row();
            }
        }

        let flags = [
            ("Disabled", node.is_disabled()),
            ("Hidden", node.is_hidden()),
            ("Read-only", node.is_read_only()),
        ];
        let mut on_flags: Vec<&str> = flags
            .iter()
            .filter(|(_, on)| *on)
            .map(|(n, _)| *n)
            .collect();
        if let Some(sel) = node.is_selected() {
            on_flags.push(if sel { "Selected" } else { "Unselected" });
        }
        if !on_flags.is_empty() {
            ui.label("Flags:");
            ui.monospace(on_flags.join(", "));
            ui.end_row();
        }

        if let Some(t) = node.toggled() {
            ui.label("Toggled:");
            ui.monospace(format!("{t:?}"));
            ui.end_row();
        }

        let child_count = node.children().len();
        if child_count > 0 {
            ui.label("Children:");
            ui.monospace(child_count.to_string());
            ui.end_row();
        }
    });
}
