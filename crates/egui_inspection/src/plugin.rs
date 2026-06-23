//! [`InspectionPlugin`] — an [`egui::Plugin`] that lets an external inspector read the
//! AccessKit tree, inject input, and capture screenshots of a running app over a simple
//! request/response protocol ([`crate::protocol`]).
//!
//! # Model
//!
//! The plugin owns a list of in-flight requests. A connection thread (or a host with its own
//! transport) submits a [`Request`] through egui's own plugin
//! handle — `ctx.with_plugin::<InspectionPlugin, _>(|p| p.submit(req, on_reply))` — passing a
//! closure that is called once with the single [`Response`], then calls `ctx.request_repaint()`
//! so an idle app wakes up to service it. The reply is produced on the UI thread inside the
//! plugin's hooks (so `on_reply` runs there too — keep it cheap, e.g. forward onto a channel),
//! which receive the [`egui::Context`] to issue repaints and viewport commands — so the plugin
//! never has to store a `Context` itself.
//!
//! [`serve`] binds a TCP listener; each accepted connection gets a thread that first writes
//! the protocol handshake, then loops reading framed [`Request`]s, submitting them, and
//! writing the framed [`Response`] back. Multiple clients are just multiple connections.
//!
//! Because egui locks each plugin only for the duration of a single hook call, a background
//! thread can take that same lock (via `with_plugin`) between hooks to enqueue work — so
//! egui's plugin handle *is* the cross-thread channel; no extra shared handle is needed.
//!
//! # Servicing
//!
//! Requests advance through a small per-request state machine across one or two frames:
//! `GetInfo` replies immediately; `GetTree` replies with the current frame's tree;
//! `Resize` / `ApplyEvents` apply their effect and reply [`Response::Done`] *after* the frame
//! has processed them (so a following `GetTree` reflects them); `GetScreenshot` dispatches a
//! viewport screenshot and replies once the resulting [`egui::Event::Screenshot`] arrives,
//! matched back to the request by a `user_data` id.
//!
//! Note that [`serve`]'s threads hold an [`egui::Context`] clone, so the context stays alive
//! for as long as the listener runs (the lifetime of the process, for a debug attach).

use std::sync::mpsc;
use std::time::Duration;

use egui::{Context, FullOutput, RawInput};

use crate::protocol::{EncodedPng, Request, Response};

/// How long [`serve`]'s connection threads wait for the UI thread before giving up. Generous:
/// a backgrounded window may not paint (and thus not service requests) for a while.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// Per-[`Request`] progress through the frame lifecycle.
#[derive(PartialEq, Eq)]
enum Phase {
    /// Just submitted by a connection thread; not yet picked up by `input_hook`.
    New,

    /// Effect applied (or nothing to apply); reply at the end of this frame.
    AwaitOutput,

    /// A screenshot was dispatched with this `user_data` id; reply when the matching
    /// [`egui::Event::Screenshot`] arrives.
    AwaitScreenshot { id: u64 },
}

struct InFlight {
    req: Request,

    /// Called once, on the UI thread, with this request's reply. `Option` so it can be moved out
    /// during `retain_mut` (which only hands out `&mut`) when the request completes.
    reply: Option<Box<dyn FnOnce(Response) + Send + Sync>>,
    phase: Phase,
}

/// An [`egui::Plugin`] that serves the inspection protocol. See the module docs.
pub struct InspectionPlugin {
    /// Requests we haven't responded to yet.
    in_flight: Vec<InFlight>,

    step: u64,

    /// Counter for screenshot `user_data` ids, so each [`egui::Event::Screenshot`] maps back
    /// to the request that asked for it.
    next_screenshot_id: u64,

    /// App label reported in [`Response::Info`].
    label: Option<String>,
}

impl InspectionPlugin {
    /// Create the plugin and register it with [`Context::add_plugin`], then call [`serve`] to
    /// listen on TCP (or feed it directly via `ctx.with_plugin(|p| p.submit(req, on_reply))`).
    pub fn new(label: Option<String>) -> Self {
        Self {
            in_flight: Vec::new(),
            step: 0,
            next_screenshot_id: 0,
            label,
        }
    }

    /// Submit an inspection [`Request`].
    ///
    /// The closure will be called later once the result comes in (for screenshot that could mean
    /// a couple frames delay).
    ///
    /// You usually call this via [`Context::with_plugin`]. You should [`Context::request_repaint`]
    /// after calling this.
    pub fn submit(
        &mut self,
        req: Request,
        on_reply: impl FnOnce(Response) + Send + Sync + 'static,
    ) {
        self.in_flight.push(InFlight {
            req,
            reply: Some(Box::new(on_reply)),
            phase: Phase::New,
        });
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

        // Match screenshot replies to the requests that asked for them, by `user_data` id. We
        // observe (don't consume) the event so the host app still receives it.
        let pixels_per_point = ctx.pixels_per_point();
        for ev in &input.events {
            let egui::Event::Screenshot {
                user_data, image, ..
            } = ev
            else {
                continue;
            };
            let Some(id) = user_data
                .data
                .as_ref()
                .and_then(|d| d.downcast_ref::<u64>())
                .copied()
            else {
                continue; // not one of ours
            };
            self.in_flight.retain_mut(|item| {
                if item.phase != (Phase::AwaitScreenshot { id }) {
                    return true;
                }
                // Downscale to the request's requested pixels-per-point (px per logical point);
                // the framebuffer is at the app's `pixels_per_point` px per point, so the scale
                // factor is their ratio. `None` means native resolution (scale 1.0).
                let scale = match item.req {
                    Request::GetScreenshot {
                        pixels_per_point: Some(requested_ppp),
                    } => requested_ppp / pixels_per_point,
                    _ => 1.0,
                };
                let png = match EncodedPng::from_color_image_scaled(image.as_ref(), scale) {
                    Ok(png) => png,
                    Err(err) => {
                        // Shouldn't happen for a valid framebuffer; surface it loudly and drop
                        // the request rather than hang on it.
                        log::error!("egui_inspection: PNG encode failed: {err}");
                        return false;
                    }
                };
                if let Some(reply) = item.reply.take() {
                    reply(Response::Screenshot(png));
                }
                false
            });
        }

        // Apply the input-side effect of new requests, dropping any that reply immediately.
        // `label`/`next_id` are pulled out so the closure doesn't borrow `self` alongside the
        // `retain_mut` borrow of `in_flight`.
        let label = self.label.clone();
        let mut next_id = self.next_screenshot_id;
        self.in_flight.retain_mut(|item| {
            if item.phase != Phase::New {
                return true;
            }
            match &item.req {
                Request::GetInfo => {
                    if let Some(reply) = item.reply.take() {
                        reply(Response::Info {
                            label: label.clone(),
                            egui_version: env!("CARGO_PKG_VERSION").to_owned(),
                        });
                    }
                    false
                }
                Request::GetTree => {
                    item.phase = Phase::AwaitOutput;
                    true
                }
                Request::ApplyEvents { events } => {
                    input.events.extend(events.iter().cloned());
                    // Reply with `Done` at the end of the frame so the agent can be sure the
                    // events were *executed* (e.g. a button click that created a file), not
                    // merely received.
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
                Request::GetScreenshot { .. } => {
                    // Dispatch now so the command lands in this frame's output and the capture
                    // is one frame sooner; the pixels arrive in a later `input_hook`. The id
                    // ties that `Event::Screenshot` back to this request.
                    let id = next_id;
                    next_id += 1;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                        id,
                    )));
                    item.phase = Phase::AwaitScreenshot { id };
                    true
                }
            }
        });
        self.next_screenshot_id = next_id;

        self.maybe_repaint(ctx);
    }

    fn output_hook(&mut self, ctx: &Context, output: &mut FullOutput) {
        self.step = self.step.saturating_add(1);
        if self.in_flight.is_empty() {
            return;
        }

        let step = self.step;
        self.in_flight
            .retain_mut(|item| match (&item.phase, &item.req) {
                (Phase::AwaitOutput, Request::GetTree) => {
                    if let Some(reply) = item.reply.take() {
                        reply(Response::Tree {
                            step,
                            pixels_per_point: output.pixels_per_point,
                            accesskit: output.platform_output.accesskit_update.clone(),
                        });
                    }
                    false
                }
                (Phase::AwaitOutput, Request::ApplyEvents { .. } | Request::Resize { .. }) => {
                    if let Some(reply) = item.reply.take() {
                        reply(Response::Done);
                    }
                    false
                }
                _ => true,
            });

        self.maybe_repaint(ctx);
    }
}

/// Attach inspection if enabled via the environment (see [`crate::bind_addr_from_env`]).
///
/// Registers an [`InspectionPlugin`] on `ctx` and starts serving on the configured address.
/// Returns `Ok(true)` when attached, `Ok(false)` when inspection is disabled.
///
/// # Errors
/// When the env-configured address can't be bound.
#[cfg(not(target_arch = "wasm32"))]
pub fn attach_from_env(ctx: &Context, label: Option<String>) -> std::io::Result<bool> {
    let Some(addr) = crate::bind_addr_from_env() else {
        return Ok(false);
    };
    ctx.add_plugin(InspectionPlugin::new(label));
    serve(ctx, &addr)?;
    Ok(true)
}

/// Bind a TCP listener at `addr` (e.g. `127.0.0.1:5719`) and accept inspector connections.
///
/// Drives the [`InspectionPlugin`] registered on `ctx`. Spawns one accept thread plus a
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
                std::thread::Builder::new()
                    .name("egui_inspection_conn".into())
                    .spawn(move || {
                        if let Err(err) = serve_connection(stream, &ctx) {
                            log::warn!("egui_inspection: connection ended: {err}");
                        }
                    })
                    .expect("failed to spawn egui_inspection connection thread");
            }
        })?;
    Ok(())
}

/// Connection handler: write the handshake, then read framed requests, submit each to the
/// plugin via the context, and write the framed response back. Returns once the client
/// disconnects.
///
/// # Errors
/// On any socket I/O failure.
#[cfg(not(target_arch = "wasm32"))]
fn serve_connection(stream: std::net::TcpStream, ctx: &Context) -> std::io::Result<()> {
    use crate::protocol::{read_message, write_handshake, write_message};

    let mut reader = std::io::BufReader::new(stream.try_clone()?);
    let mut writer = std::io::BufWriter::new(stream);

    // Identify ourselves and our protocol version before any framed messages.
    write_handshake(&mut writer)?;

    loop {
        let req: Request = match read_message(&mut reader) {
            Ok(req) => req,
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()), // client gone
            Err(err) => return Err(err),
        };

        let (tx, rx) = mpsc::channel();
        let registered = ctx
            .with_plugin::<InspectionPlugin, _>(|p| {
                p.submit(req, move |resp| {
                    let _ = tx.send(resp);
                });
            })
            .is_some();
        if !registered {
            return write_message(
                &mut writer,
                &Response::Error {
                    message: "egui_inspection plugin not registered".to_owned(),
                },
            );
        }
        // Wake the (possibly idle) UI loop so it services the request.
        ctx.request_repaint();
        let resp = rx.recv_timeout(REQUEST_TIMEOUT).unwrap_or_else(|_| {
            // Almost always means the app isn't painting — e.g. the window is occluded or
            // minimized, which on most platforms stops rendering. Surface it loudly.
            log::error!(
                "egui_inspection: request timed out after {REQUEST_TIMEOUT:?}; the app is not \
                 painting (is the window occluded or minimized?)"
            );
            Response::Error {
                message: "request timed out — the app is not painting; bring its window to the \
                          foreground"
                    .to_owned(),
            }
        });
        write_message(&mut writer, &resp)?;
    }
}
