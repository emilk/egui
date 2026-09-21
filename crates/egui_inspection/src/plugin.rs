//! [`InspectionPlugin`] — an [`egui::Plugin`] that lets an external inspector read the
//! AccessKit tree, inject input, and capture screenshots of a running app over a simple
//! request/response protocol ([`crate::protocol`]).
//!
//! # Model
//!
//! The plugin owns a list of in-flight requests. A connection thread (or a host with its own
//! transport) submits a [`Request`] through egui's own plugin
//! handle — `ctx.with_plugin::<InspectionPlugin, _>(|p| p.submit(req, on_reply))` — passing a
//! closure that is called once with the single [`Response`], then sends
//! [`egui::ViewportCommand::RequestPaintWhileHidden`] so an idle app wakes up and runs its ui
//! to service the request — even an app whose window is minimized or occluded, which an
//! integration would otherwise let sleep. The reply is produced on the UI thread inside the
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
//! `Resize` / `ApplyEvents` / `DropFile` apply their effect and reply [`Response::Done`] *after*
//! the frame has processed them (so a following `GetTree` reflects them); `GetScreenshot`
//! dispatches a viewport screenshot and replies once the screenshot callback has delivered the
//! pixels, matched back to the request by an id.
//!
//! Note that [`serve`]'s threads hold an [`egui::Context`] clone, so the context stays alive
//! for as long as the listener runs (the lifetime of the process, for a debug attach).

use core::time::Duration;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
};

#[cfg(target_arch = "wasm32")]
use core::{future::Future, pin::Pin};

use egui::{ColorImage, Context, FullOutput, RawInput, mutex::Mutex};

use crate::protocol::{EncodedPng, Request, Response};

/// How long [`serve`]'s connection threads wait for the UI thread before giving up. Generous:
/// an app that is busy, or slow to paint, may take a while to service a request.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// Per-[`Request`] progress through the frame lifecycle.
#[derive(PartialEq, Eq)]
enum Phase {
    /// Just submitted by a connection thread; not yet picked up by `input_hook`.
    New,

    /// Effect applied (or nothing to apply); reply at the end of this frame.
    AwaitOutput,

    /// A screenshot was dispatched with this id; reply when the matching callback has
    /// delivered the pixels.
    AwaitScreenshot { id: u64 },

    /// Watching for the app to go idle.
    Settle { steps_taken: u64, max_steps: u64 },
}

#[derive(Debug)]
struct MemoryFile {
    path: PathBuf,
    bytes: Vec<u8>,
}

impl egui::DroppedFile for MemoryFile {
    fn path(&self) -> &Path {
        &self.path
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn bytes(&self) -> Result<Vec<u8>, String> {
        Ok(self.bytes.clone())
    }

    #[cfg(target_arch = "wasm32")]
    fn bytes_async(&self) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + '_>> {
        Box::pin(async { Ok(self.bytes.clone()) })
    }
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

    /// Counter for screenshot ids, so each delivered screenshot maps back to the request that
    /// asked for it.
    next_screenshot_id: u64,

    /// Screenshots delivered by [`egui::Context::request_screenshot`] callbacks, tagged with the
    /// id of the request that asked for them. Written from whichever thread the renderer
    /// completes the capture on, drained by `input_hook`.
    received_screenshots: Arc<Mutex<Vec<(u64, Arc<ColorImage>)>>>,

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
            received_screenshots: Default::default(),
            label,
        }
    }

    /// Submit an inspection [`Request`].
    ///
    /// The closure will be called later once the result comes in (for screenshot that could mean
    /// a couple frames delay).
    ///
    /// You usually call this via [`Context::with_plugin`]. You should then ask for a frame with
    /// [`egui::ViewportCommand::RequestPaintWhileHidden`], so that an app that is idle, or whose
    /// window is hidden, wakes up and serves the request.
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
    ///
    /// Asks for the frame even if the window is hidden: a minimized or occluded app runs no
    /// pass at all otherwise, and an inspector is usually attached to an app in the background.
    fn maybe_repaint(&self, ctx: &Context) {
        // Don't repaint if there's only a `Request::Settle`.
        if self
            .in_flight
            .iter()
            .any(|item| !matches!(item.req, Request::Settle { .. }))
        {
            request_frame(ctx);
        }
    }
}

/// Ask for one run of the ui, painted, whether or not the root window is visible.
///
/// This both wakes an idle app and overrides the integration's skipping of hidden windows.
/// Every request needs it: a screenshot needs the painted pixels, the widget tree is what the
/// pass produces, and injected input is only applied by a pass.
fn request_frame(ctx: &Context) {
    ctx.send_viewport_cmd_to(
        egui::ViewportId::ROOT,
        egui::ViewportCommand::RequestPaintWhileHidden,
    );
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

        // Match delivered screenshots to the requests that asked for them, by id.
        let pixels_per_point = ctx.pixels_per_point();
        for (id, image) in core::mem::take(&mut *self.received_screenshots.lock()) {
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
        let received_screenshots = Arc::clone(&self.received_screenshots);
        let mut next_id = self.next_screenshot_id;
        self.in_flight.retain_mut(|item| {
            if item.phase != Phase::New {
                return true;
            }
            match &mut item.req {
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
                Request::DropFile { filename, bytes } => {
                    input.dropped_files.push(Arc::new(MemoryFile {
                        path: core::mem::take(filename).into(),
                        bytes: core::mem::take(bytes),
                    }));
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
                    // ties the delivered screenshot back to this request.
                    let id = next_id;
                    next_id += 1;
                    let received = Arc::clone(&received_screenshots);
                    ctx.request_screenshot(move |image| {
                        received.lock().push((id, image));
                    });
                    item.phase = Phase::AwaitScreenshot { id };
                    true
                }
                Request::Settle { max_steps } => {
                    item.phase = Phase::Settle {
                        steps_taken: 0,
                        max_steps: *max_steps,
                    };
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

        let immediate_repaint = output
            .viewport_output
            .values()
            .any(|viewport| viewport.repaint_delay == Duration::ZERO);

        let step = self.step;
        self.in_flight
            .retain_mut(|item| match (&mut item.phase, &item.req) {
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
                (
                    Phase::AwaitOutput,
                    Request::ApplyEvents { .. } | Request::DropFile { .. } | Request::Resize { .. },
                ) => {
                    if let Some(reply) = item.reply.take() {
                        reply(Response::Done);
                    }
                    false
                }
                (
                    Phase::Settle {
                        steps_taken,
                        max_steps,
                    },
                    Request::Settle { .. },
                ) => {
                    *steps_taken += 1;
                    let steps_exceeded = *steps_taken >= *max_steps;
                    if !immediate_repaint || steps_exceeded {
                        if let Some(reply) = item.reply.take() {
                            reply(Response::Settled {
                                settled: !immediate_repaint,
                                steps: *steps_taken,
                            });
                        }
                        false
                    } else {
                        true
                    }
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
        // Wake the (possibly idle) UI loop so it services the request, hidden window and all.
        request_frame(ctx);
        let resp = rx.recv_timeout(REQUEST_TIMEOUT).unwrap_or_else(|_| {
            // The app is running no passes at all — it may be blocked, or its integration may
            // not paint hidden windows. Surface it loudly.
            log::error!(
                "egui_inspection: request timed out after {REQUEST_TIMEOUT:?}; the app is not \
                 painting"
            );
            Response::Error {
                message: "request timed out — the app is not painting".to_owned(),
            }
        });
        write_message(&mut writer, &resp)?;
    }
}

#[cfg(test)]
mod tests {
    use egui::{ViewportCommand, ViewportId};

    use super::*;

    /// Run one pass, the way an integration does, and return what it asks of the window.
    fn pass(ctx: &Context) -> Vec<ViewportCommand> {
        let mut output = ctx.run_ui(RawInput::default(), |_| {});
        let commands = output.viewport_output[&ViewportId::ROOT].commands.clone();
        output.textures_delta.clear();
        commands
    }

    fn submit(ctx: &Context, req: Request) {
        ctx.with_plugin::<InspectionPlugin, _>(|p| p.submit(req, |_| {}))
            .expect("the plugin is registered");
    }

    fn wants_paint(commands: &[ViewportCommand]) -> bool {
        commands.contains(&ViewportCommand::RequestPaintWhileHidden)
    }

    #[test]
    fn an_in_flight_request_asks_for_a_frame_even_while_hidden() {
        let ctx = Context::default();
        ctx.add_plugin(InspectionPlugin::new(None));
        assert!(
            !wants_paint(&pass(&ctx)),
            "An idle app should let a hidden window sleep"
        );

        // A screenshot takes more than one pass, so the plugin must keep asking:
        submit(
            &ctx,
            Request::GetScreenshot {
                pixels_per_point: None,
            },
        );
        assert!(
            wants_paint(&pass(&ctx)),
            "A screenshot of a hidden window needs the pass that paints it"
        );
    }

    #[test]
    fn a_file_drop_is_appended_once_and_acknowledged_after_the_frame() {
        for bytes in [Vec::new(), vec![0, 127, 128, 255]] {
            let ctx = Context::default();
            ctx.add_plugin(InspectionPlugin::new(None));
            let (tx, rx) = mpsc::channel();
            ctx.with_plugin::<InspectionPlugin, _>(|p| {
                p.submit(
                    Request::DropFile {
                        filename: "données.bin".to_owned(),
                        bytes: bytes.clone(),
                    },
                    move |response| tx.send(response).expect("The receiver is alive"),
                );
            })
            .expect("The plugin is registered");

            let existing: egui::DroppedFileHandle = Arc::new(MemoryFile {
                path: "existing.txt".into(),
                bytes: vec![42],
            });
            let input = RawInput {
                dropped_files: vec![Arc::clone(&existing)],
                ..Default::default()
            };
            let mut dropped = None;
            ctx.run_ui(input, |ui| {
                assert!(
                    matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)),
                    "A drop must not be acknowledged before the UI processes it"
                );
                ui.input(|input| {
                    assert_eq!(input.raw.dropped_files.len(), 2);
                    assert!(
                        Arc::ptr_eq(&input.raw.dropped_files[0], &existing),
                        "An existing drop must be preserved"
                    );
                    dropped = Some(Arc::clone(&input.raw.dropped_files[1]));
                });
            })
            .drop_without_applying_deltas();

            assert!(matches!(rx.try_recv(), Ok(Response::Done)), "Expected Done");
            assert!(
                matches!(rx.try_recv(), Err(mpsc::TryRecvError::Disconnected)),
                "The request must complete with exactly one reply"
            );
            let file = dropped.expect("The UI received the file");
            assert_eq!(file.path(), Path::new("données.bin"));
            #[cfg(not(target_arch = "wasm32"))]
            let contents = file.bytes();
            #[cfg(target_arch = "wasm32")]
            let contents = {
                let mut future = file.bytes_async();
                let mut cx = core::task::Context::from_waker(core::task::Waker::noop());
                let core::task::Poll::Ready(contents) = future.as_mut().poll(&mut cx) else {
                    panic!("An in-memory file should be ready to read");
                };
                contents
            };
            assert_eq!(contents, Ok(bytes));

            ctx.run_ui(RawInput::default(), |ui| {
                assert!(
                    ui.input(|input| input.raw.dropped_files.is_empty()),
                    "A drop must not be repeated in the next frame"
                );
            })
            .drop_without_applying_deltas();
        }
    }

    #[test]
    fn settling_does_not_keep_a_hidden_window_awake() {
        // `Settle` waits for the app to go idle; asking for frames would defeat it.
        let ctx = Context::default();
        ctx.add_plugin(InspectionPlugin::new(None));
        submit(&ctx, Request::Settle { max_steps: 100 });
        assert!(!wants_paint(&pass(&ctx)));
    }
}
