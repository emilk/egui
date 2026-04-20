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
use kittest_inspector::{read_message, write_message, Frame, HarnessMessage, InspectorReply};

use accesskit::{Node, NodeId, Rect as AkRect};

/// Internal worker → UI message.
enum WorkerEvent {
    Frame(Box<Frame>),
    Disconnected,
}

/// UI → worker message: "you may send `Continue` to the harness now".
/// Carries any egui events captured in Control mode that the harness should queue.
type ReleaseTx = mpsc::Sender<Vec<egui::Event>>;
type ReleaseRx = mpsc::Receiver<Vec<egui::Event>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayState {
    Playing,
    Paused,
}

/// Fast-forward state for the ⏭ Next button.
#[derive(Debug, Clone, Copy)]
enum SkipState {
    Inactive,
    /// Auto-release every incoming frame until `call_site_line` differs from this value.
    UntilNewCallLine(Option<u32>),
}

impl SkipState {
    fn is_active(self) -> bool {
        matches!(self, Self::UntilNewCallLine(_))
    }
}

fn main() -> eframe::Result<()> {
    // Cross-process single-instance guard. If another inspector is already running, block
    // here until that window closes. Held for the lifetime of `_lock`; the OS releases the
    // flock when the file descriptor is dropped on exit.
    let _lock = acquire_single_instance_lock();

    let (worker_tx, worker_rx) = mpsc::channel::<WorkerEvent>();
    let (release_tx, release_rx) = mpsc::channel::<Vec<egui::Event>>();

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
                let Ok(events) = release_rx.recv() else {
                    return;
                };
                if let Err(err) = write_message(&mut writer, &InspectorReply::Continue { events }) {
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
    /// Every frame the harness has ever sent, in order. Supports back/forward replay.
    history: Vec<Frame>,
    /// Index into `history` of the currently-displayed frame.
    view_index: usize,
    /// `Frame::step` currently uploaded to `current_texture` — used to decide whether the
    /// texture needs regenerating when `view_index` changes.
    textured_step: Option<u64>,
    current_texture: Option<egui::TextureHandle>,
    connected: bool,
    /// Currently hovered widget (cleared every frame, set during central-panel paint).
    hovered_node: Option<NodeId>,
    /// Last clicked widget (sticky).
    selected_node: Option<NodeId>,
    /// When on, pointer + keyboard events are forwarded to the harness.
    control_enabled: bool,
    /// Events accumulated since the last release; drained when we send Continue.
    queued_events: Vec<egui::Event>,
    /// Set when the viewed frame changes; the Source section consumes it to scroll once.
    scroll_pending: bool,
    /// While `UntilNewCallLine`, auto-release every incoming frame until we see one with a
    /// different `call_site_line` — i.e. until the test moves past the current runner call.
    skip: SkipState,
    /// Screen rect of the rendered image from the previous frame. We hit-test against this
    /// at the start of the next `ui()` (before panels render) so the details tree can see
    /// `hovered_node` in the same frame as the image highlight.
    last_image_rect: Option<egui::Rect>,
    /// Display-pixel-per-physical-pixel ratio from the previous frame.
    last_image_scale: f32,
    /// Transient status line (e.g. "Copied to /tmp/...") shown next to the Copy-GIF button.
    status_message: Option<String>,
}

impl InspectorApp {
    /// The frame currently being displayed. `None` only before the first frame ever arrives.
    fn view_frame(&self) -> Option<&Frame> {
        self.history.get(self.view_index)
    }

    /// True when `view_index` points at the most recent frame (so new arrivals keep scrolling
    /// the view forward).
    fn is_live_view(&self) -> bool {
        !self.history.is_empty() && self.view_index + 1 == self.history.len()
    }

    fn set_view_index(&mut self, idx: usize) {
        let idx = idx.min(self.history.len().saturating_sub(1));
        if idx != self.view_index {
            self.view_index = idx;
            self.scroll_pending = true;
        }
    }
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
            history: Vec::new(),
            view_index: 0,
            textured_step: None,
            current_texture: None,
            connected: true,
            hovered_node: None,
            selected_node: None,
            control_enabled: false,
            queued_events: Vec::new(),
            scroll_pending: false,
            skip: SkipState::Inactive,
            last_image_rect: None,
            last_image_scale: 1.0,
            status_message: None,
        }
    }

    /// Hit-test the current cursor position against the cached image rect + the viewed
    /// frame's accesskit bounds and set `hovered_node`. Called at the top of `ui()` so the
    /// tree (rendered before the image) picks up the same hover state in this frame.
    fn hit_test_pointer(&mut self, ctx: &egui::Context) {
        if self.control_enabled {
            return; // In control mode we forward events, we don't inspect on hover.
        }
        let (Some(image_rect), Some(frame)) = (self.last_image_rect, self.view_frame()) else {
            return;
        };
        let Some(update) = frame.accesskit.as_ref() else {
            return;
        };
        let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
            return;
        };
        if !image_rect.contains(pos) {
            return;
        }
        let f = (frame.pixels_per_point * self.last_image_scale) as f64;
        let lx = ((pos.x - image_rect.min.x) as f64) / f;
        let ly = ((pos.y - image_rect.min.y) as f64) / f;
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
        self.hovered_node = best.map(|(id, _)| id);
    }

    fn pump_worker(&mut self) {
        while let Ok(event) = self.worker_rx.try_recv() {
            match event {
                WorkerEvent::Frame(frame) => {
                    let new_call_line = frame.source.as_ref().and_then(|s| s.call_site_line);
                    let was_live = self.is_live_view() || self.history.is_empty();
                    self.history.push(*frame);
                    if was_live {
                        self.view_index = self.history.len() - 1;
                    }
                    self.worker_waiting = true;

                    // If we're fast-forwarding to the next `run()` call, stop once the
                    // call_site line differs from the one we started from.
                    let still_skipping = matches!(
                        self.skip,
                        SkipState::UntilNewCallLine(from) if new_call_line == from
                    );
                    if still_skipping {
                        // Don't auto-scroll / flash for in-between frames we're about to blow
                        // past; the user will see the first settled frame at the new call.
                    } else {
                        self.skip = SkipState::Inactive;
                        // Only scroll the source panel for the frame the user will actually
                        // see (i.e. when we're following the live edge).
                        if was_live {
                            self.scroll_pending = true;
                        }
                    }
                }
                WorkerEvent::Disconnected => {
                    self.connected = false;
                    self.worker_waiting = false;
                    self.skip = SkipState::Inactive;
                }
            }
        }
    }

    /// (Re-)upload `view_frame()`'s pixels to `current_texture` if the texture is missing or
    /// represents a different step than what we're viewing.
    fn ensure_texture_uploaded(&mut self, ctx: &egui::Context) {
        let Some(frame) = self.view_frame() else {
            return;
        };
        if self.textured_step == Some(frame.step) {
            return;
        }
        let size = [frame.width as usize, frame.height as usize];
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &frame.rgba);
        let texture = ctx.load_texture("kittest_inspector_frame", color_image, Default::default());
        self.textured_step = Some(frame.step);
        self.current_texture = Some(texture);
    }

    fn send_release(&mut self) {
        if !self.worker_waiting {
            return;
        }
        let events = std::mem::take(&mut self.queued_events);
        if self.release_tx.send(events).is_ok() {
            self.worker_waiting = false;
        }
    }
}

impl eframe::App for InspectorApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.pump_worker();
        self.ensure_texture_uploaded(&ctx);
        // Reset hover each frame — either the pre-hit-test below (using the cached image
        // rect from the previous frame) or the tree's own hover detection, or the central
        // panel's live hit-test will set it again.
        self.hovered_node = None;
        self.hit_test_pointer(&ctx);

        controls_panel(self, ui);
        details_panel(self, ui);
        central_panel(self, ui);

        // End-of-frame auto-release policy:
        // - Fast-forwarding to the next `run()` call: always release.
        // - Control mode: stay blocked, but advance one step whenever the user generates events
        //   (each click / keypress = one harness step).
        // - Otherwise, Playing mode runs freely; Paused mode waits for Next/Play/Step.
        let auto_release = if self.skip.is_active() {
            true
        } else if self.control_enabled {
            !self.queued_events.is_empty()
        } else {
            self.play_state == PlayState::Playing
        };
        if self.worker_waiting && auto_release {
            self.send_release();
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}

fn controls_panel(app: &mut InspectorApp, ui: &mut egui::Ui) {
    egui::Panel::top("controls").show_inside(ui, |ui| {
        ui.horizontal(|ui| {
            let playing = app.play_state == PlayState::Playing;
            let play_response = ui
                .add_enabled_ui(!app.control_enabled, |ui| {
                    ui.selectable_label(playing, "▶ Play")
                })
                .inner
                .on_disabled_hover_text("Disabled while Control mode is on");
            if play_response.clicked() {
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
                .add_enabled(can_step, egui::Button::new("⏩ Step"))
                .on_hover_text("Advance one harness internal step")
                .clicked()
            {
                app.send_release();
            }
            if ui
                .add_enabled(can_step, egui::Button::new("⏭ Next"))
                .on_hover_text(
                    "Fast-forward until the test reaches the next `run()` / `step()` call",
                )
                .clicked()
            {
                // "From" is the *live* frame's call_site — the harness is blocked there, not
                // at wherever the user is currently browsing in history.
                let current_line = app
                    .history
                    .last()
                    .and_then(|f| f.source.as_ref())
                    .and_then(|s| s.call_site_line);
                app.skip = SkipState::UntilNewCallLine(current_line);
                app.send_release();
            }

            ui.separator();

            // History navigation.
            let total = app.history.len();
            let can_back = app.view_index > 0;
            let can_forward = app.view_index + 1 < total;
            if ui
                .add_enabled(can_back, egui::Button::new("⏴"))
                .on_hover_text("Previous frame in history")
                .clicked()
            {
                app.set_view_index(app.view_index.saturating_sub(1));
            }
            if ui
                .add_enabled(can_forward, egui::Button::new("⏵"))
                .on_hover_text("Next frame in history")
                .clicked()
            {
                app.set_view_index(app.view_index + 1);
            }
            if ui
                .add_enabled(can_forward, egui::Button::new("⏩ Live"))
                .on_hover_text("Jump to the newest frame (follow live updates)")
                .clicked()
            {
                app.set_view_index(total.saturating_sub(1));
            }
            if total > 0 {
                // Both the slider value and the label are 1-indexed for display.
                let mut scrub = app.view_index + 1;
                let response = ui.add(
                    egui::Slider::new(&mut scrub, 1..=total)
                        .text(format!("/ {total}"))
                        .clamping(egui::SliderClamping::Always),
                );
                if response.changed() {
                    app.set_view_index(scrub.saturating_sub(1));
                }
            }

            if ui
                .add_enabled(total > 0, egui::Button::new("📋 Copy as GIF"))
                .on_hover_text(
                    "Encode the whole history as a GIF, write it to the system temp dir, \
                     and copy the resulting path to the clipboard.",
                )
                .clicked()
            {
                let message = match save_history_as_gif(&app.history, 10.0) {
                    Ok(path) => {
                        ui.ctx().copy_text(path.display().to_string());
                        format!("Copied path to clipboard: {}", path.display())
                    }
                    Err(err) => format!("Failed to save GIF: {err}"),
                };
                eprintln!("kittest_inspector: {message}");
                app.status_message = Some(message);
            }
            if let Some(msg) = app.status_message.as_deref() {
                ui.weak(msg);
            }

            ui.separator();

            let prev_control = app.control_enabled;
            if !app.connected {
                // Nothing to drive if the harness is gone.
                app.control_enabled = false;
            }
            ui.add_enabled_ui(app.connected, |ui| {
                ui.checkbox(&mut app.control_enabled, "🎮 Control")
                    .on_hover_text(
                        "Forward pointer and keyboard events on the rendered frame to the harness",
                    )
                    .on_disabled_hover_text("Harness disconnected");
            });
            if prev_control && !app.control_enabled {
                app.queued_events.clear();
            }

            ui.separator();
            ui.label(if app.connected {
                if app.worker_waiting {
                    "harness blocked"
                } else {
                    "harness running"
                }
            } else {
                "harness disconnected"
            });
        });
    });
}

fn details_panel(app: &mut InspectorApp, ui: &mut egui::Ui) {
    egui::Panel::right("details")
        .resizable(true)
        .default_size(380.0)
        .show_inside(ui, |ui| {
            let Some(frame) = app.view_frame().cloned() else {
                ui.weak("Waiting for frames...");
                return;
            };

            // The Source view sits in its own resizable top panel so the user can drop it out
            // of the way when they want more room for the widget / AccessKit sections below.
            egui::Panel::top("details_source")
                .resizable(true)
                .default_size(280.0)
                .show_inside(ui, |ui| {
                    ui.heading("Source");
                    let scroll_pending = std::mem::take(&mut app.scroll_pending);
                    source_section(ui, &frame, scroll_pending);
                });

            egui::ScrollArea::vertical().show(ui, |ui| {
                // Make long values (file paths, labels, stringified values in the widget
                // details grid, accesskit node names…) wrap inside the fixed-width side panel
                // instead of overflowing to the right.
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

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
                            let node_count = frame.accesskit.as_ref().map_or(0, |u| u.nodes.len());
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

                egui::CollapsingHeader::new("AccessKit tree")
                    .default_open(false)
                    .show(ui, |ui| {
                        if let Some(update) = &frame.accesskit {
                            accesskit_tree(
                                ui,
                                update,
                                &mut app.selected_node,
                                &mut app.hovered_node,
                            );
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
        let Some(frame) = app.view_frame().cloned() else {
            return;
        };

        let physical = tex.size_vec2(); // physical pixels of the rendered frame
        let avail = ui.available_size();
        let scale = (avail.x / physical.x)
            .min(avail.y / physical.y)
            .clamp(0.05, 1.0);
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
        // Cache the image placement so the next frame's `hit_test_pointer` can run before
        // the tree is rendered and keep the two in sync.
        app.last_image_rect = Some(image_rect);
        app.last_image_scale = scale;

        // logical_point → screen_position:
        //     screen = image_rect.min + ak_rect * pixels_per_point * scale
        let logical_to_screen = |r: AkRect| -> egui::Rect {
            let f = frame.pixels_per_point * scale;
            egui::Rect::from_min_max(
                image_rect.min + egui::vec2(r.x0 as f32 * f, r.y0 as f32 * f),
                image_rect.min + egui::vec2(r.x1 as f32 * f, r.y1 as f32 * f),
            )
        };

        if app.control_enabled {
            // In Control mode clicks/hovers drive the harness, not the inspector.
            forward_events(
                app,
                ui,
                image_rect,
                frame.pixels_per_point,
                scale,
                &response,
            );
        } else {
            // Inspection mode: hover was already resolved in `hit_test_pointer` at the top
            // of `ui()` so the tree and the image stay in sync — we only need to handle the
            // click here.
            if response.clicked() {
                app.selected_node = app.hovered_node;
            }

            let painter = ui.painter_at(image_rect);
            if let Some(update) = &frame.accesskit {
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
        }
    });
}

/// Inspect the inspector's own input events and forward those relevant to the harness.
///
/// Pointer events only forward when their position is inside the rendered-image rect and their
/// coordinates are translated to harness logical space. Keyboard / text events always forward.
fn forward_events(
    app: &mut InspectorApp,
    ui: &egui::Ui,
    image_rect: egui::Rect,
    pixels_per_point: f32,
    scale: f32,
    image_response: &egui::Response,
) {
    let to_logical = |pos: egui::Pos2| -> egui::Pos2 {
        let f = pixels_per_point * scale;
        egui::pos2(
            (pos.x - image_rect.min.x) / f,
            (pos.y - image_rect.min.y) / f,
        )
    };

    let input_events = ui.ctx().input(|i| i.events.clone());
    for ev in input_events {
        match ev {
            egui::Event::PointerMoved(pos) if image_rect.contains(pos) => {
                app.queued_events
                    .push(egui::Event::PointerMoved(to_logical(pos)));
            }
            egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers,
            } if image_rect.contains(pos) => {
                app.queued_events.push(egui::Event::PointerButton {
                    pos: to_logical(pos),
                    button,
                    pressed,
                    modifiers,
                });
            }
            egui::Event::PointerGone => {
                app.queued_events.push(egui::Event::PointerGone);
            }
            mw @ egui::Event::MouseWheel { .. } if image_response.hovered() => {
                app.queued_events.push(mw);
            }
            ev @ (egui::Event::Text(_)
            | egui::Event::Key { .. }
            | egui::Event::Copy
            | egui::Event::Cut
            | egui::Event::Paste(_)
            | egui::Event::Ime(_)) => {
                app.queued_events.push(ev);
            }
            _ => {}
        }
    }
}

fn kv_grid(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id)
        .num_columns(2)
        .striped(true)
        .show(ui, body);
}

/// Render the "Source" section: the test file (topmost common ancestor across the call and
/// its events), with the relevant lines highlighted and (once per new frame) the view
/// scrolled to them.
fn source_section(ui: &mut egui::Ui, frame: &kittest_inspector::Frame, scroll_pending: bool) {
    let Some(source) = &frame.source else {
        ui.weak("No source location for this frame.");
        return;
    };

    ui.horizontal(|ui| {
        ui.monospace(shorten_path(&source.path));
        if let Some(line) = source.call_site_line {
            ui.weak(format!("(producer: line {line})"));
        }
    });

    let Some(contents) = source.contents.as_deref() else {
        ui.weak(format!("(couldn't read {})", source.path));
        return;
    };

    let call_site_line = source.call_site_line;
    let event_lines: std::collections::HashSet<u32> = source.event_lines.iter().copied().collect();
    let focus_line = call_site_line.or_else(|| source.event_lines.first().copied());

    // Semi-transparent tints so the highlight works in both light and dark themes without
    // darkening the text. Alpha ~72/255 keeps the underlying text fully legible.
    let call_bg = egui::Color32::from_rgba_unmultiplied(80, 160, 255, 72);
    let event_bg = egui::Color32::from_rgba_unmultiplied(255, 180, 60, 72);

    let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
    let lines: Vec<&str> = contents.lines().collect();
    let total_height = lines.len() as f32 * row_height;

    // Estimated monospace advance width. For fixed-pitch fonts (like Hack) the ratio between
    // character height and advance is ~0.55; being slightly generous avoids clipping.
    let char_width = row_height * 0.6_f32;
    let longest_chars = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as f32;
    let gutter_width = char_width * 5.0 + ui.spacing().item_spacing.x; // "{:>4} " column
    let content_width: f32 = gutter_width + char_width * longest_chars + 16.0;

    // Expand to fill the enclosing (resizable) panel — the user's drag on the panel handle
    // determines how tall the source view is.
    let scroll_area = egui::ScrollArea::both().auto_shrink([false, false]);
    // `show_viewport` lets us decide ourselves which rows to render + lets us reason in the
    // content's *virtual* coordinate space. That means we can build a target rect for the
    // focus line whether or not it's currently visible, and `scroll_to_rect` will animate
    // the scroll area towards it smoothly.
    scroll_area.show_viewport(ui, |ui, viewport| {
        let row_width = content_width.max(viewport.width());
        ui.set_height(total_height);
        ui.set_width(row_width);
        let content_top = ui.min_rect().top();
        let content_left = ui.min_rect().left();
        let start = (viewport.min.y / row_height).floor().max(0.0) as usize;
        let end = ((viewport.max.y / row_height).ceil() as usize)
            .min(lines.len())
            .max(start);

        for (idx, line) in lines.iter().enumerate().take(end).skip(start) {
            let line_no = idx as u32 + 1;
            let y = idx as f32 * row_height;
            let row_rect = egui::Rect::from_min_size(
                egui::pos2(content_left, content_top + y),
                egui::vec2(row_width, row_height),
            );
            let is_call = Some(line_no) == call_site_line;
            let is_event = event_lines.contains(&line_no);
            let bg = if is_call {
                Some(call_bg)
            } else if is_event {
                Some(event_bg)
            } else {
                None
            };
            let mut row_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(row_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            source_line_row(&mut row_ui, line_no, line, bg, row_rect);
        }

        if scroll_pending && let Some(focus) = focus_line {
            let y = focus.saturating_sub(1) as f32 * row_height;
            let target = egui::Rect::from_min_size(
                egui::pos2(content_left, content_top + y),
                egui::vec2(1.0, row_height),
            );
            ui.scroll_to_rect(target, Some(egui::Align::Center));
        }
    });
}

fn source_line_row(
    ui: &mut egui::Ui,
    line_no: u32,
    text: &str,
    bg: Option<egui::Color32>,
    row_rect: egui::Rect,
) {
    if let Some(color) = bg {
        ui.painter().rect_filled(row_rect, 2.0, color);
    }
    ui.add(egui::Label::new(
        egui::RichText::new(format!("{line_no:>4} "))
            .monospace()
            .weak(),
    ));
    ui.add(
        egui::Label::new(egui::RichText::new(text).monospace())
            .wrap_mode(egui::TextWrapMode::Extend),
    );
}

/// Shorten a `rustc`-reported path for display — keep the last two components so we show
/// `tests/menu.rs` instead of a long absolute path, while still disambiguating.
fn shorten_path(path: &str) -> String {
    let components: Vec<&str> = path.split(['/', '\\']).collect();
    if components.len() <= 2 {
        path.to_owned()
    } else {
        let n = components.len();
        format!("{}/{}", components[n - 2], components[n - 1])
    }
}

/// Render the accesskit tree recursively, similar in style to the egui demo's `inspection_ui`
/// — collapsible parents with their children indented below, leaves as selectable labels.
fn accesskit_tree(
    ui: &mut egui::Ui,
    update: &accesskit::TreeUpdate,
    selected: &mut Option<NodeId>,
    hovered: &mut Option<NodeId>,
) {
    use std::collections::{HashMap, HashSet};

    let nodes: HashMap<NodeId, &Node> = update.nodes.iter().map(|(id, n)| (*id, n)).collect();

    // Prefer the tree's declared root. If this update doesn't carry tree-level info (diff-only
    // updates can omit it), fall back to any node that no other node lists as a child.
    let root = update.tree.as_ref().map(|t| t.root).or_else(|| {
        let mut children: HashSet<NodeId> = HashSet::new();
        for (_, node) in &update.nodes {
            for c in node.children() {
                children.insert(*c);
            }
        }
        update.nodes.iter().map(|(id, _)| *id).find(|id| !children.contains(id))
    });

    match root {
        Some(root_id) => render_ak_node(ui, root_id, &nodes, selected, hovered),
        None => {
            // Shouldn't normally happen; degrade to a flat list.
            for (id, _) in &update.nodes {
                render_ak_node(ui, *id, &nodes, selected, hovered);
            }
        }
    }
}

fn render_ak_node(
    ui: &mut egui::Ui,
    id: NodeId,
    nodes: &std::collections::HashMap<NodeId, &Node>,
    selected: &mut Option<NodeId>,
    hovered: &mut Option<NodeId>,
) {
    let Some(node) = nodes.get(&id).copied() else {
        ui.weak(format!("(missing {:?})", id.0));
        return;
    };
    let role = format!("{:?}", node.role());
    let text = match node.label().or_else(|| node.value()) {
        Some(label) if !label.is_empty() => format!("{role}  {label:?}"),
        _ => role,
    };
    // Both the image's hovered state and the tree's selection light up the same row — a row
    // shown highlighted in the tree corresponds to the rect drawn on the image.
    let highlight = *selected == Some(id) || *hovered == Some(id);
    let children = node.children();

    if children.is_empty() {
        let response = ui.selectable_label(highlight, text);
        if response.clicked() {
            *selected = Some(id);
        }
        if response.hovered() {
            *hovered = Some(id);
        }
        return;
    }

    let header_id = ui.make_persistent_id(("ak_node", id.0));
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), header_id, true)
        .show_header(ui, |ui| {
            let response = ui.selectable_label(highlight, text);
            if response.clicked() {
                *selected = Some(id);
            }
            if response.hovered() {
                *hovered = Some(id);
            }
        })
        .body(|ui| {
            for child_id in children {
                render_ak_node(ui, *child_id, nodes, selected, hovered);
            }
        });
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

/// Encode the entire history as a looping GIF, write it to a timestamped file in the system
/// temp dir, and return the path. Mirrors the recorder's GIF behaviour: animation plays at
/// `frame_rate`, last frame held for one second so the loop point is obvious.
fn save_history_as_gif(
    history: &[Frame],
    frame_rate: f32,
) -> Result<std::path::PathBuf, String> {
    use image::codecs::gif::{GifEncoder, Repeat};

    if history.is_empty() {
        return Err("history is empty".into());
    }

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    // Stable-across-processes temp path is fine here: each invocation wants a fresh file.
    #[expect(clippy::disallowed_methods)]
    let path = std::env::temp_dir().join(format!("kittest_inspector_{ts}.gif"));

    let file = std::fs::File::create(&path)
        .map_err(|err| format!("couldn't create {}: {err}", path.display()))?;
    let writer = std::io::BufWriter::new(file);
    let mut encoder = GifEncoder::new(writer);
    encoder
        .set_repeat(Repeat::Infinite)
        .map_err(|err| format!("set_repeat: {err}"))?;

    let denom = frame_rate
        .max(0.1)
        .round()
        .clamp(1.0, u32::MAX as f32) as u32;
    let frame_delay = image::Delay::from_numer_denom_ms(1000, denom);
    let hold_delay = image::Delay::from_numer_denom_ms(1000, 1);

    let last_idx = history.len() - 1;
    for (i, frame) in history.iter().enumerate() {
        let Some(buffer) =
            image::RgbaImage::from_raw(frame.width, frame.height, frame.rgba.clone())
        else {
            return Err(format!(
                "frame {i} has inconsistent rgba size for {}×{}",
                frame.width, frame.height
            ));
        };
        let delay = if i == last_idx {
            hold_delay
        } else {
            frame_delay
        };
        let anim_frame = image::Frame::from_parts(buffer, 0, 0, delay);
        encoder
            .encode_frame(anim_frame)
            .map_err(|err| format!("encode frame {i}: {err}"))?;
    }

    Ok(path)
}

/// Try to acquire a cross-process exclusive lock on a well-known file so that only one
/// inspector window can be open on the machine at a time. Blocks here (before we open any
/// windows or touch stdio beyond this stderr line) if another inspector is already running.
fn acquire_single_instance_lock() -> Option<std::fs::File> {
    use fs4::fs_std::FileExt;

    // We specifically need a stable, cross-process path here — tempfile's per-process dir
    // can't serve as a system-wide mutex.
    #[expect(clippy::disallowed_methods)]
    let path = std::env::temp_dir().join("kittest_inspector.lock");

    let file = match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
    {
        Ok(f) => f,
        Err(err) => {
            eprintln!(
                "kittest_inspector: couldn't open lock file {}: {err} (running without single-instance guard)",
                path.display()
            );
            return None;
        }
    };

    match FileExt::lock_exclusive(&file) {
        Ok(()) => Some(file),
        Err(err) => {
            eprintln!("kittest_inspector: failed to acquire lock: {err}");
            None
        }
    }
}
