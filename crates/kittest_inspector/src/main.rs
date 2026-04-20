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

fn main() -> eframe::Result<()> {
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
                if let Err(err) =
                    write_message(&mut writer, &InspectorReply::Continue { events })
                {
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
    /// When on, pointer + keyboard events are forwarded to the harness.
    control_enabled: bool,
    /// Events accumulated since the last release; drained when we send Continue.
    queued_events: Vec<egui::Event>,
    /// Set when a new frame arrives; the Source section consumes it to scroll once per frame.
    scroll_pending: bool,
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
            control_enabled: false,
            queued_events: Vec::new(),
            scroll_pending: false,
        }
    }

    fn pump_worker(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.worker_rx.try_recv() {
            match event {
                WorkerEvent::Frame(frame) => {
                    self.received_count += 1;
                    self.upload_frame(ctx, &frame);
                    // Keep the selection sticky across frames (same NodeId may still exist).
                    self.current_frame = Some(*frame);
                    self.worker_waiting = true;
                    self.scroll_pending = true;
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
        let events = std::mem::take(&mut self.queued_events);
        if self.release_tx.send(events).is_ok() {
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

        // End-of-frame auto-release policy:
        // - Control mode: stay blocked, but advance one step whenever the user generates events
        //   (each click / keypress = one harness step).
        // - Otherwise, Playing mode runs freely; Paused mode waits for Next/Play.
        let auto_release = if self.control_enabled {
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
                .add_enabled(can_step, egui::Button::new("⏭ Next"))
                .on_hover_text("Advance one harness step")
                .clicked()
            {
                app.send_release();
            }

            ui.separator();

            let prev_control = app.control_enabled;
            ui.checkbox(&mut app.control_enabled, "🎮 Control")
                .on_hover_text(
                    "Forward pointer and keyboard events on the rendered frame to the harness",
                );
            if prev_control && !app.control_enabled {
                app.queued_events.clear();
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

                let scroll_pending = std::mem::take(&mut app.scroll_pending);
                egui::CollapsingHeader::new("Source")
                    .default_open(true)
                    .show(ui, |ui| {
                        source_section(ui, &frame, scroll_pending);
                    });

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

        if app.control_enabled {
            // In Control mode clicks/hovers drive the harness, not the inspector.
            forward_events(app, ui, image_rect, frame.pixels_per_point, scale, &response);
        } else {
            // Inspection mode: hit test (smallest containing widget wins) + draw overlays.
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

    let scroll_area = egui::ScrollArea::both()
        .auto_shrink([false, false])
        .max_height(320.0);
    // `show_viewport` lets us decide ourselves which rows to render + lets us reason in the
    // content's *virtual* coordinate space. That means we can build a target rect for the
    // focus line whether or not it's currently visible, and `scroll_to_rect` will animate
    // the scroll area towards it smoothly.
    scroll_area.show_viewport(ui, |ui, viewport| {
        ui.set_height(total_height);
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
                egui::vec2(ui.available_width(), row_height),
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

        if scroll_pending
            && let Some(focus) = focus_line
        {
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
