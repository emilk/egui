//! [`InspectionPlugin`] — an [`egui::Plugin`] that lets an external inspector read the
//! AccessKit tree, inject input, and capture screenshots of a running app over a simple
//! request/response protocol ([`crate::protocol`]).
//!
//! # Model
//!
//! The plugin owns a list of in-flight requests. Any thread holding the [`egui::Context`]
//! submits a [`Request`] through egui's own plugin handle —
//! `ctx.with_plugin::<InspectionPlugin, _>(|p| p.submit(req))` — which appends it and returns
//! a channel to await the single [`Response`] on, then calls `ctx.request_repaint()` so an
//! idle app wakes up to service it. The reply is produced on the UI thread inside the
//! plugin's hooks, which receive the [`egui::Context`] to issue repaints and viewport
//! commands — so the plugin never has to store a `Context` itself.
//!
//! Two front doors feed requests in:
//! - [`serve`] binds a TCP listener; each accepted connection gets a thread that reads
//!   framed [`Request`]s, submits them, and writes the framed [`Response`] back. Multiple
//!   clients are just multiple connections.
//! - A host that owns its own transport (e.g. `re_mcp` over gRPC) holds the app's
//!   [`egui::Context`] and submits the same way.
//!
//! Because egui locks each plugin only for the duration of a single hook call, a background
//! thread can take that same lock (via `with_plugin`) between hooks to enqueue work — so
//! egui's plugin handle *is* the cross-thread channel; no extra shared handle is needed.
//!
//! # Servicing
//!
//! Requests advance through a small per-request state machine across one or two frames:
//! `Info` replies immediately; `GetTree` replies with the current frame's tree; `Resize` /
//! `HandleEvents` apply their effect and ack *after* the frame has processed them (so a
//! following `GetTree` reflects them); `Screenshot` dispatches a viewport screenshot and
//! replies once the resulting [`egui::Event::Screenshot`] arrives.
//!
//! Note that [`serve`]'s threads hold a [`egui::Context`] clone, so the context stays alive
//! for as long as the listener runs (the lifetime of the process, for a debug attach).

use std::sync::mpsc;
use std::time::Duration;

use egui::{Context, FullOutput, RawInput};

use crate::protocol::{PROTOCOL_VERSION, Request, Response};

/// How long [`serve`]'s connection threads wait for the UI thread before giving up. Generous:
/// a backgrounded window may not paint (and thus not service requests) for a while.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// Per-request progress through the frame lifecycle.
enum Phase {
    /// Freshly submitted; not yet seen by `input_hook`.
    New,
    /// Effect applied (or nothing to apply); reply at the end of this frame.
    AwaitOutput,
    /// Screenshot dispatched (in `input_hook`); reply when the `Event::Screenshot` pixels arrive.
    AwaitScreenshotPixels,
}

struct InFlight {
    req: Request,
    reply: mpsc::Sender<Response>,
    phase: Phase,
}

/// An [`egui::Plugin`] that serves the inspection protocol. See the module docs.
pub struct InspectionPlugin {
    in_flight: Vec<InFlight>,
    step: u64,
    /// App label reported in [`Response::Info`].
    label: Option<String>,
}

impl InspectionPlugin {
    /// Create the plugin and register it with [`Context::add_plugin`], then call [`serve`] to
    /// listen on TCP (or feed it directly via `ctx.with_plugin(|p| p.submit(req))`).
    pub fn new(label: Option<String>) -> Self {
        Self {
            in_flight: Vec::new(),
            step: 0,
            label,
        }
    }

    /// Submit a request; returns a channel that receives its single reply once the UI thread
    /// services it. Call this through [`Context::with_plugin`] so it runs under egui's plugin
    /// lock, then `request_repaint` and await the receiver *after* the lock is released.
    pub fn submit(&mut self, req: Request) -> mpsc::Receiver<Response> {
        let (tx, rx) = mpsc::channel();
        self.in_flight.push(InFlight {
            req,
            reply: tx,
            phase: Phase::New,
        });
        rx
    }

    /// While requests are still in flight, keep the UI loop spinning — reactive apps would
    /// otherwise go idle between hooks before a screenshot round-trips.
    fn maybe_repaint(&self, ctx: &Context) {
        if !self.in_flight.is_empty() {
            ctx.request_repaint();
        }
    }
}

impl egui::Plugin for InspectionPlugin {
    fn debug_name(&self) -> &'static str {
        "egui_inspection"
    }

    fn setup(&mut self, ctx: &Context) {
        // The inspector describes the UI via the AccessKit tree.
        ctx.enable_accesskit();
    }

    fn input_hook(&mut self, ctx: &Context, input: &mut RawInput) {
        // Nothing in flight → idle frame, do no work.
        if self.in_flight.is_empty() {
            return;
        }

        // Capture any screenshot reply produced for a previous `Screenshot` request. We
        // observe (don't consume) so the host app still receives it.
        let mut screenshot: Option<(u32, u32, Vec<u8>)> = None;
        for ev in &input.events {
            if let egui::Event::Screenshot { image, .. } = ev {
                let (w, h) = (image.size[0] as u32, image.size[1] as u32);
                let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
                match crate::encode_png(w, h, &rgba) {
                    Ok(png) => screenshot = Some((w, h, png)),
                    Err(err) => log::warn!("egui_inspection: PNG encode failed: {err}"),
                }
                break;
            }
        }
        if let Some((w, h, png)) = screenshot {
            self.in_flight.retain_mut(|item| {
                if !matches!(item.phase, Phase::AwaitScreenshotPixels) {
                    return true;
                }
                let _ = item.reply.send(Response::Screenshot {
                    width: w,
                    height: h,
                    png: png.clone(),
                });
                false
            });
        }

        // Apply the input-side effect of new requests, dropping any that reply immediately.
        // `label` is cloned out so the closure doesn't borrow `self` alongside the
        // `retain_mut` borrow of `in_flight`.
        let label = self.label.clone();
        self.in_flight.retain_mut(|item| {
            if !matches!(item.phase, Phase::New) {
                return true;
            }
            match &item.req {
                Request::Info => {
                    let _ = item.reply.send(Response::Info {
                        protocol_version: PROTOCOL_VERSION,
                        label: label.clone(),
                    });
                    false
                }
                Request::GetTree => {
                    item.phase = Phase::AwaitOutput;
                    true
                }
                Request::HandleEvents { events } => {
                    input.events.extend(events.iter().cloned());
                    item.phase = Phase::AwaitOutput;
                    true
                }
                Request::Resize { width, height } => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                        *width as f32,
                        *height as f32,
                    )));
                    item.phase = Phase::AwaitOutput;
                    true
                }
                Request::Screenshot => {
                    // Dispatch now so the command lands in this frame's output and the
                    // capture is one frame sooner; the pixels arrive in a later `input_hook`.
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                        egui::UserData::default(),
                    ));
                    item.phase = Phase::AwaitScreenshotPixels;
                    true
                }
            }
        });

        self.maybe_repaint(ctx);
    }

    fn output_hook(&mut self, ctx: &Context, output: &mut FullOutput) {
        self.step = self.step.saturating_add(1);
        if self.in_flight.is_empty() {
            return;
        }

        let step = self.step;
        self.in_flight.retain_mut(|item| match (&item.phase, &item.req) {
            (Phase::AwaitOutput, Request::GetTree) => {
                let _ = item.reply.send(Response::Tree {
                    step,
                    pixels_per_point: output.pixels_per_point,
                    accesskit: output.platform_output.accesskit_update.clone(),
                });
                false
            }
            (Phase::AwaitOutput, Request::HandleEvents { .. } | Request::Resize { .. }) => {
                let _ = item.reply.send(Response::Ack);
                false
            }
            _ => true,
        });

        self.maybe_repaint(ctx);
    }
}

/// If inspection is enabled via the environment (see [`crate::bind_addr_from_env`]), register
/// an [`InspectionPlugin`] on `ctx` and start serving on the configured address. Returns
/// `Ok(true)` when attached, `Ok(false)` when inspection is disabled.
///
/// # Errors
/// When the env-configured address can't be bound.
pub fn attach_from_env(ctx: &Context, label: Option<String>) -> std::io::Result<bool> {
    let Some(addr) = crate::bind_addr_from_env() else {
        return Ok(false);
    };
    ctx.add_plugin(InspectionPlugin::new(label));
    serve(ctx, &addr)?;
    Ok(true)
}

/// Bind a TCP listener at `addr` (e.g. `127.0.0.1:5719`) and accept inspector connections,
/// driving the [`InspectionPlugin`] registered on `ctx`. Spawns one accept thread plus a
/// thread per connection (detached — they live for the process).
///
/// Binding a non-loopback address exposes the inspection port (and thus full control of the
/// app, plus its screenshots) to the network with no authentication — a warning is logged.
///
/// # Errors
/// When `addr` can't be parsed or bound.
#[cfg(not(target_arch = "wasm32"))]
pub fn serve(ctx: &Context, addr: &str) -> std::io::Result<()> {
    use std::net::{TcpListener, ToSocketAddrs as _};

    let resolved = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| std::io::Error::other(format!("no address resolved from {addr:?}")))?;
    let listener = TcpListener::bind(resolved)?;
    let bound = listener.local_addr()?;
    if bound.ip().is_loopback() {
        log::info!("egui_inspection: listening on {bound}");
    } else {
        log::warn!(
            "egui_inspection: listening on {bound} — the inspection port is reachable from \
             the network with NO authentication; anyone who can reach it can drive the app \
             and read its screen"
        );
    }

    let ctx = ctx.clone();
    std::thread::Builder::new()
        .name("egui_inspection_accept".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let ctx = ctx.clone();
                let _ = std::thread::Builder::new()
                    .name("egui_inspection_conn".into())
                    .spawn(move || serve_connection(stream, &ctx));
            }
        })?;
    Ok(())
}

/// On wasm there is no TCP listener; serving is unsupported.
///
/// # Errors
/// Always, on wasm.
#[cfg(target_arch = "wasm32")]
pub fn serve(_ctx: &Context, _addr: &str) -> std::io::Result<()> {
    Err(std::io::Error::other(
        "egui_inspection: TCP serving is not supported on wasm",
    ))
}

/// Connection handler: read framed requests, submit each to the plugin via the context, and
/// write the framed response back. Returns on EOF or any I/O error.
#[cfg(not(target_arch = "wasm32"))]
fn serve_connection(stream: std::net::TcpStream, ctx: &Context) {
    use crate::protocol::{read_message, write_message};

    let Ok(write_stream) = stream.try_clone() else {
        return;
    };
    let mut reader = std::io::BufReader::new(stream);
    let mut writer = std::io::BufWriter::new(write_stream);

    loop {
        let req: Request = match read_message(&mut reader) {
            Ok(r) => r,
            Err(_) => return, // EOF / decode error → client gone
        };
        let Some(rx) = ctx.with_plugin::<InspectionPlugin, _>(|p| p.submit(req)) else {
            let _ = write_message(
                &mut writer,
                &Response::Error {
                    message: "egui_inspection plugin not registered".to_owned(),
                },
            );
            return;
        };
        // Wake the (possibly idle) UI loop so it services the request.
        ctx.request_repaint();
        let resp = rx.recv_timeout(REQUEST_TIMEOUT).unwrap_or(Response::Error {
            message: "egui inspection request timed out (app not painting?)".to_owned(),
        });
        if write_message(&mut writer, &resp).is_err() {
            return;
        }
    }
}
