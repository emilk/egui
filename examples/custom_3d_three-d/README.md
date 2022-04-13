This demo shows how to embed 3D rendering using [`three-d`](https://github.com/asny/three-d) in `eframe`.

Any 3D library built on top of [`glow`](https://github.com/grovesNL/glow) can be used in `eframe`.

Alternatively you can render 3D stuff to a texture and display it using [`egui::Ui::image`].

If you are content of having egui sit on top of a 3D background, take a look at:

* [`bevy_egui`](https://github.com/mvlabat/bevy_egui)
* [`three-d`](https://github.com/asny/three-d)



```sh
cargo run -p custom_3d_three-d
```
