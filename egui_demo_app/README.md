# egui demo app
This app demonstrates [`egui`](https://github.com/emilk/egui/) and [`eframe`](https://github.com/emilk/egui/tree/master/eframe).

View the demo app online at <https://egui.rs>.

Run it locally with `cargo run --release -p egui_demo_app`.

`egui_demo_app` can be compiled to WASM and viewed in a browser locally with:

```sh
./sh/start_server.sh &
./sh/build_demo_web.sh --fast --open
```

`egui_demo_app` uses [`egui_demo_lib`](https://github.com/emilk/egui/tree/master/egui_demo_lib).
