use serde_diff::Diff;
use std::net::{TcpListener, TcpStream};
use tungstenite::{accept, Message, WebSocket};

fn main() {
    let mut client: WebSocket<TcpStream>;
    let mut current_pixels_per_point: f32 = 2.0;
    let mut latest_raw_input: egui::RawInput = egui::RawInput::default();
    let mut context = egui::Context::default();
    let mut previous_full_output = egui::FullOutput::default();
    let mut count = 0;

    let server = TcpListener::bind("127.0.0.1:3012").unwrap();
    let stream = server.incoming().next().unwrap().unwrap();
    client = accept(stream).unwrap();

    loop {
        let message = &client.read_message();
        if let Ok(content) = message {
            let msg_content = content.clone().into_text();
            if let Ok(msg) = msg_content {
                let received_message = serde_json::from_str(&msg);
                if let Ok(content) = received_message {
                    match content {
                        eframe::RemoteRenderingMessageType::Connect(pixels_per_point) => {
                            current_pixels_per_point = pixels_per_point;
                        }
                        eframe::RemoteRenderingMessageType::PixelsPerPoint(pixels_per_point) => {
                            current_pixels_per_point = pixels_per_point;
                        }
                        eframe::RemoteRenderingMessageType::RawInput(raw_input) => {
                            latest_raw_input = raw_input;
                        }
                    }
                }
            }
        }

        let ctx = context.clone();
        ctx.set_pixels_per_point(current_pixels_per_point);
        ctx.begin_frame(latest_raw_input.clone());
        egui::Window::new("Hello world!")
            .default_pos(egui::pos2(100.0, 0.0))
            .show(&ctx, |ui| {
                ui.label(format!("Count: {:?}", count));
            });

        ctx.request_repaint();
        let full_output = ctx.end_frame();
        context = ctx;
        count += 1;
        let old_full_output = previous_full_output.clone();
        let serialized_full_output = serde_json::to_string(&full_output).unwrap();
        previous_full_output = full_output.clone();
        if let Ok(diff) = serialized_full_output {
            let _ = client.write_message(Message::Text(diff));
        }
    }
}
