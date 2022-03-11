//! This demo shows how to embed 3D rendering using [`glow`](https://github.com/grovesNL/glow) in `eframe`.
//!
//! This is very advanced usage, and you need to be careful.
//!
//! If you want an easier way to show 3D graphics with egui, take a look at:
//! * [`bevy_egui`](https://github.com/mvlabat/bevy_egui)
//! * [`three-d`](https://github.com/asny/three-d)

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{egui, epi};

use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Default)]
struct MyApp {
    rotating_triangle: Arc<Mutex<Option<RotatingTriangle>>>,
    angle: f32,
}

impl epi::App for MyApp {
    fn name(&self) -> &str {
        "Custom 3D painting inside an egui window"
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Here is some 3D stuff:");

            egui::ScrollArea::both().show(ui, |ui| {
                egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
                    self.custom_painting(ui);
                });
                ui.label("Drag to rotate!");
            });
        });

        let mut frame = egui::Frame::window(&*ctx.style());
        frame.fill = frame.fill.linear_multiply(0.5); // transparent
        egui::Window::new("3D stuff in a window")
            .frame(frame)
            .show(ctx, |ui| {
                self.custom_painting(ui);
            });
    }
}

impl MyApp {
    fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let (rect, response) =
            ui.allocate_exact_size(egui::Vec2::splat(256.0), egui::Sense::drag());

        self.angle += response.drag_delta().x * 0.01;

        let angle = self.angle;
        let rotating_triangle = self.rotating_triangle.clone();

        let callback = egui::epaint::PaintCallback {
            rect,
            callback: std::sync::Arc::new(move |render_ctx| {
                if let Some(painter) = render_ctx.downcast_ref::<egui_glow::Painter>() {
                    let mut rotating_triangle = rotating_triangle.lock();
                    let rotating_triangle = rotating_triangle
                        .get_or_insert_with(|| RotatingTriangle::new(painter.gl()));
                    rotating_triangle.paint(painter.gl(), angle);
                } else {
                    eprintln!("Can't do custom painting because we are not using a glow context");
                }
            }),
        };
        ui.painter().add(callback);
    }
}

struct RotatingTriangle {
    program: glow::Program,
    vertex_array: glow::VertexArray,
}

impl RotatingTriangle {
    fn new(gl: &glow::Context) -> Self {
        use glow::HasContext as _;

        let shader_version = if cfg!(target_arch = "wasm32") {
            "#version 300 es"
        } else {
            "#version 410"
        };

        unsafe {
            let program = gl.create_program().expect("Cannot create program");

            let (vertex_shader_source, fragment_shader_source) = (
                r#"
                    const vec2 verts[3] = vec2[3](
                        vec2(0.0, 1.0),
                        vec2(-1.0, -1.0),
                        vec2(1.0, -1.0)
                    );
                    const vec4 colors[3] = vec4[3](
                        vec4(1.0, 0.0, 0.0, 1.0),
                        vec4(0.0, 1.0, 0.0, 1.0),
                        vec4(0.0, 0.0, 1.0, 1.0)
                    );
                    out vec4 v_color;
                    uniform float u_angle;
                    void main() {
                        v_color = colors[gl_VertexID];
                        gl_Position = vec4(verts[gl_VertexID], 0.0, 1.0);
                        gl_Position.x *= cos(u_angle);
                    }
                "#,
                r#"
                    precision mediump float;
                    in vec4 v_color;
                    out vec4 out_color;
                    void main() {
                        out_color = v_color;
                    }
                "#,
            );

            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_shader_source),
                (glow::FRAGMENT_SHADER, fragment_shader_source),
            ];

            let shaders: Vec<_> = shader_sources
                .iter()
                .map(|(shader_type, shader_source)| {
                    let shader = gl
                        .create_shader(*shader_type)
                        .expect("Cannot create shader");
                    gl.shader_source(shader, &format!("{}\n{}", shader_version, shader_source));
                    gl.compile_shader(shader);
                    if !gl.get_shader_compile_status(shader) {
                        panic!("{}", gl.get_shader_info_log(shader));
                    }
                    gl.attach_shader(program, shader);
                    shader
                })
                .collect();

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            let vertex_array = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");

            Self {
                program,
                vertex_array,
            }
        }
    }

    // TODO: figure out how to call this in a nice way
    #[allow(unused)]
    fn destroy(self, gl: &glow::Context) {
        use glow::HasContext as _;
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vertex_array);
        }
    }

    fn paint(&self, gl: &glow::Context, angle: f32) {
        use glow::HasContext as _;
        unsafe {
            gl.use_program(Some(self.program));
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "u_angle").as_ref(),
                angle,
            );
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
    }
}

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(MyApp::default()), options);
}
