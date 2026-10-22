# egui-winit

[![Latest version](https://img.shields.io/crates/v/egui-winit.svg)](https://crates.io/crates/egui-winit)
[![Documentation](https://docs.rs/egui-winit/badge.svg)](https://docs.rs/egui-winit)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

This crates provides bindings between [`egui`](https://github.com/emilk/egui) and [`winit`](https://crates.io/crates/winit).

The library translates winit events to egui, handled copy/paste, updates the cursor, open links clicked in egui, etc.

## winit 0.31 migration preview

This draft branch migrates the native adapters and eframe to winit 0.31. Its
pinned dependency forks are review prototypes, and must be replaced with agreed
upstream dependencies before merging or publishing a release.

Custom integrations pass the active event loop to
`State::on_window_event(window, event_loop, event)`. `EguiGlow::on_window_event`
and `EguiGlow::run` also take the active event loop. Native windows are
`Arc<dyn winit::window::Window>`; winit uses surface sizes and positions rather
than the earlier inner-size APIs. The external-event-loop examples show the new
non-generic event loop. iOS application creators must own their captured state
because winit owns the application for the event loop's lifetime.

On Wayland, accepted file offers provide a MIME-only hover indicator. Names and
contents are requested only after Drop; the matching asynchronous response adds
files to `RawInput::dropped_files` at the drop position. Leave and cancellation
clear the indicator. The receiver accepts Copy actions, local file URIs, at most
256 paths and 1 MiB of transfer data, with eight pending requests and a ten-second
deadline. Portal file capabilities are retrieved after Drop through the optional
`wayland-portal` feature on `egui-winit` or `eframe` (off by default).
This prototype adds a libdbus build requirement when enabled, and does not grant
general access to the host filesystem.

Text copy/paste uses winit's data device on Wayland so clipboard initialization
cannot replace its drag-and-drop device. Paste is asynchronous; menu actions and
keyboard shortcuts both use `State::request_paste`. Existing native image
clipboard fallbacks remain in place. Other window backends retain their native
transfer path.

The local Linux validation includes an isolated Wayland protocol server,
overlapping transfers, cancellation, unsupported actions, remote and NUL paths,
oversized and stalled writers, repeated clipboard paste, and a real desktop
portal. A downstream VectorCraft test also uses GTK file-list serialization and
an installed isolated Flatpak, checks that original files are inaccessible before
the drop, then verifies imported images and artboard sizes. This is transport and
application-import evidence. A contributor subsequently confirmed a physical Ubuntu
Desktop drop in the integrated 0.36 downstream application, after explicitly
granting read-only Desktop access to that application's Flatpak. The Desktop
source offered ordinary URIs, not portal capabilities; that local permission is
not supplied by this adapter. Physical Nautilus gestures and Apple/Android runtime
validation remain outstanding. Private-session-bus Flatpak tests disturbed the
live document portal and were disabled; reproduce those tests in a separate OS
test session, not alongside the user's applications.

### Review dependencies and checks

- Transfer limits and portal API direction: [winit #4790](https://github.com/rust-windowing/winit/issues/4790).
- MIME parsing: [winit #4788](https://github.com/rust-windowing/winit/pull/4788).
- Dropped-offer lifetime: [SCTK #546](https://github.com/Smithay/client-toolkit/pull/546).
- Current window error type: [glutin #1767](https://github.com/rust-windowing/glutin/pull/1767).
- Accessibility release coordination: [AccessKit #654](https://github.com/AccessKit/accesskit/issues/654).
- Earlier migration discussion: [egui #7731](https://github.com/emilk/egui/pull/7731).

Local checks on Linux (Rust 1.95): `egui-winit` library tests (including IME restoration and recycled-window-ID cleanup); eframe's
threaded/coalesced-wakeup regression; strict all-target eframe Clippy with both
renderers, accessibility, clipboard and portal support; direct `egui-winit` and
eframe WebAssembly compilation; formatting and the custom source linter.
Strict all-feature library documentation builds; optional-feature-only documentation
can still warn about existing inspection/persistence links when those features are off. The
full workspace test run completed with five targets failing image snapshot comparisons
(demo app/library, kittest, regression widgets). The reference images were not
changed. Remaining feature checks and a baseline snapshot comparison are recorded
in the PR. The dependency audit reports advisories/licenses OK but rejects
prototype Git sources and duplicate dependency versions. The full check script
and all-platform runtime checks have not passed as a complete matrix. Keep this draft until dependency and platform
review is complete; this document does not claim release readiness.


Shutdown handling owns the application handler in run-on-demand mode so native
termination can drop/save it even without returning to the caller. The macOS root
window Destroyed path also saves externally pumped applications. This was reviewed
against the platform lifecycle code; macOS runtime confirmation remains required.
IME enable requests retain desired purpose/cursor settings while disabled and
clear per-window cached settings when the owning integration state is dropped.
