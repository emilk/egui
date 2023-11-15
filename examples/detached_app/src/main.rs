#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::time::Instant;

use eframe::{egui, DetachedResult};
use winit::{
    event::{ElementState, Event, WindowEvent},
    event_loop::ControlFlow,
};

fn main() {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };
    let event_loop = eframe::EventLoopBuilder::<eframe::UserEvent>::with_user_event().build();

    // A detached window managed by eframe
    let mut detached_app = eframe::run_detached_native(
        "My egui App",
        &event_loop,
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    );
    // Winit window managed by the application
    let winit_window = winit::window::WindowBuilder::new()
        .build(&event_loop)
        .unwrap();

    let mut next_paint = Instant::now();
    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = match event {
            // Check first for events on the managed window
            Event::WindowEvent { window_id, event } if window_id == winit_window.id() => {
                match event {
                    WindowEvent::CloseRequested => ControlFlow::Exit,
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        ..
                    } => {
                        winit_window.set_maximized(!winit_window.is_maximized());
                        ControlFlow::WaitUntil(next_paint)
                    }
                    _ => ControlFlow::WaitUntil(next_paint),
                }
            }
            // Otherwise, let eframe process the event
            _ => match detached_app.on_event(&event, event_loop).unwrap() {
                DetachedResult::Exit => ControlFlow::Exit,
                DetachedResult::UpdateNext => ControlFlow::Poll,
                DetachedResult::UpdateAt(_next_paint) => {
                    next_paint = _next_paint;
                    ControlFlow::WaitUntil(next_paint)
                }
            },
        }
    });
}

struct MyApp {
    name: String,
    age: u32,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "Arthur".to_owned(),
            age: 42,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
            ui.horizontal(|ui| {
                let name_label = ui.label("Your name: ");
                ui.text_edit_singleline(&mut self.name)
                    .labelled_by(name_label.id);
            });
            ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
            if ui.button("Click each year").clicked() {
                self.age += 1;
            }
            ui.label(format!("Hello '{}', age {}", self.name, self.age));
        });
    }
}
