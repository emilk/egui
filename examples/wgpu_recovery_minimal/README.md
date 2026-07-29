# Minimal WGPU recovery test

Small manual test for WGPU recovery after device/surface loss, especially after disconnecting and reconnecting an RDP session.

Run it from the repository root:

```powershell
cargo run -p wgpu_recovery_minimal
```

The app writes a local log file named `wgpu_recovery_minimal.log` next to the executable.
The same log is also mirrored to stderr while running from a terminal.

What to try:

1. Interact with the text box, slider, and button.
2. Disconnect the RDP session, then reconnect.
3. Verify the UI still responds and the frame counter keeps increasing.

Useful log markers:

- `heartbeat frame=...`: `eframe::App::ui` is still running.
- `Windows session restored; requesting WGPU recovery`: eframe observed a session restore.
- `Recreating WGPU render state after Windows session restore`: eframe requested recovery.
- `Renderer changed: ...`: eframe observed a new WGPU render state.

Run with logs:

```powershell
$env:RUST_LOG="wgpu_recovery_minimal=info,eframe=warn,egui_wgpu=warn,wgpu_core=warn,wgpu_hal=warn"
cargo run -p wgpu_recovery_minimal
```
