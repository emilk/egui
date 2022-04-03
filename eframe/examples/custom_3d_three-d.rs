//! This demo shows how to embed 3D rendering using [`three-d`](https://github.com/asny/three-d) in `eframe`.
//!
//! Any 3D library built on top of [`glow`](https://github.com/grovesNL/glow) can be used in `eframe`.
//!
//! Alternatively you can render 3D stuff to a texture and display it using [`egui::Ui::image`].
//!
//! If you are content of having egui sit on top of a 3D background, take a look at:
//!
//! * [`bevy_egui`](https://github.com/mvlabat/bevy_egui)
//! * [`three-d`](https://github.com/asny/three-d)

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(550.0, 610.0)),
        multisampling: 8,
        ..Default::default()
    };
    eframe::run_native(
        "Custom 3D painting in eframe!",
        options,
        Box::new(|cc| Box::new(MyApp::new(cc))),
    );
}

struct MyApp {
    angle: f32,
}

impl MyApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self { angle: 0.2 }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::widgets::global_dark_light_mode_buttons(ui);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("The triangle is being painted using ");
                ui.hyperlink_to("three-d", "https://github.com/asny/three-d");
                ui.label(".");
            });

            egui::ScrollArea::both().show(ui, |ui| {
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    self.custom_painting(ui);
                });
                ui.label("Drag to rotate!");
            });
        });
    }
}

impl MyApp {
    fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let (rect, response) =
            ui.allocate_exact_size(egui::Vec2::splat(512.0), egui::Sense::drag());

        self.angle += response.drag_delta().x * 0.01;

        // Clone locals so we can move them into the paint callback:
        let angle = self.angle;

        let callback = egui::PaintCallback {
            rect,
            callback: std::sync::Arc::new(move |info, render_ctx| {
                if let Some(painter) = render_ctx.downcast_ref::<egui_glow::Painter>() {
                    with_three_d_context(painter.gl(), |three_d| {
                        paint_with_three_d(three_d, info, angle);
                    });
                } else {
                    eprintln!("Can't do custom painting because we are not using a glow context");
                }
            }),
        };
        ui.painter().add(callback);
    }
}

/// We get a [`glow::Context`] from `eframe`, but we want a [`three_d::Context`].
///
/// Sadly we can't just create a [`three_d::Context`] in [`MyApp::new`] and pass it
/// to the [`egui::PaintCallback`] because [`three_d::Context`] isn't `Send+Sync`, which
/// [`egui::PaintCallback`] is.
fn with_three_d_context<R>(
    gl: &std::rc::Rc<glow::Context>,
    f: impl FnOnce(&three_d::Context) -> R,
) -> R {
    use std::cell::RefCell;
    thread_local! {
        pub static THREE_D: RefCell<Option<three_d::Context>> = RefCell::new(None);
    }

    THREE_D.with(|three_d| {
        let mut three_d = three_d.borrow_mut();
        let three_d =
            three_d.get_or_insert_with(|| three_d::Context::from_gl_context(gl.clone()).unwrap());
        f(three_d)
    })
}

fn paint_with_three_d(three_d: &three_d::Context, info: &egui::PaintCallbackInfo, angle: f32) {
    // Based on https://github.com/asny/three-d/blob/master/examples/triangle/src/main.rs
    use three_d::*;

    let viewport = Viewport {
        x: info.viewport_left_px().round() as _,
        y: info.viewport_from_bottom_px().round() as _,
        width: info.viewport_width_px().round() as _,
        height: info.viewport_height_px().round() as _,
    };

    let camera = Camera::new_perspective(
        three_d,
        viewport,
        vec3(0.0, 0.0, 2.0),
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        degrees(45.0),
        0.1,
        10.0,
    )
    .unwrap();

    // Create a CPU-side mesh consisting of a single colored triangle
    let positions = vec![
        vec3(0.5, -0.5, 0.0),  // bottom right
        vec3(-0.5, -0.5, 0.0), // bottom left
        vec3(0.0, 0.5, 0.0),   // top
    ];
    let colors = vec![
        Color::new(255, 0, 0, 255), // bottom right
        Color::new(0, 255, 0, 255), // bottom left
        Color::new(0, 0, 255, 255), // top
    ];
    let cpu_mesh = CpuMesh {
        positions: Positions::F32(positions),
        colors: Some(colors),
        ..Default::default()
    };

    // Construct a model, with a default color material, thereby transferring the mesh data to the GPU
    let mut model = Model::new(three_d, &cpu_mesh).unwrap();

    // Set the current transformation of the triangle
    model.set_transformation(Mat4::from_angle_y(radians(angle)));

    // Render the triangle with the color material which uses the per vertex colors defined at construction
    model.render(&camera, &[]).unwrap();
}
