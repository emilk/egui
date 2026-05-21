# egui_inspection

Wire protocol and `egui::Plugin` for live inspection of running egui apps and
kittest harnesses.

Two layers:

- **`protocol`** (default feature): length-prefixed MessagePack messages used by
  `egui_kittest`'s inspector, the external `kittest_inspector` UI, and the
  `egui_kittest_mcp` server.

- **`plugin`** (opt-in): an `egui::Plugin` implementation that streams frames +
  AccessKit tree updates to an inspector over a unix domain socket and applies
  received `InspectorCommand`s back into the running app. Auto-attaches when
  `EGUI_INSPECTION_SOCKET` is set.
