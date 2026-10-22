Repro for inspecting an app whose window is hidden (minimized or occluded).

```sh
cargo run -p test_inspection_hidden_window
```

The app serves the `egui_inspection` protocol and runs an inspector client in the same process.
Minimize the window, or cover it completely, and watch what the requests do.
