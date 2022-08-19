# egui demo app
This app demonstrates [`egui`](https://github.com/emilk/egui/) and [`eframe`](https://github.com/emilk/egui/tree/master/eframe).

View the demo app online at <https://egui.rs>.

Run it locally with `cargo run --release -p egui_demo_app`.

`egui_demo_app` can be compiled to WASM and viewed in a browser locally using [Trunk](https://trunkrs.dev/).

First install trunk with `cargo install --locked trunk`.

Run `trunk serve` to build and serve on `http://127.0.0.1:8080`. It will rebuild automatically if you edit the project.

```sh
./sh/start_server.sh &
./sh/build_demo_web.sh --open
```

`egui_demo_app` uses [`egui_demo_lib`](https://github.com/emilk/egui/tree/master/egui_demo_lib).


## Running with `wgpu` backend
`(cd egui_demo_app && cargo r --features wgpu)`
