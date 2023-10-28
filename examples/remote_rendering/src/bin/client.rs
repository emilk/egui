//use serde_diff::Apply;
use serde_diff::Apply;
use std::env;
use std::net::TcpStream;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1100.0, 700.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Debug Client",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
        true,
    )
}

fn connect_to_server(pixels_per_point: f32) -> WebSocket<MaybeTlsStream<TcpStream>> {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        panic!("Must provide server url as command line argument: cargo run -- [url]");
    }
    let url = &args[1];
    let (mut socket, _) = connect(url).expect("Can't connect to server");
    let connect_message = eframe::RemoteRenderingMessageType::Connect(pixels_per_point);
    let serialized_connect_message = serde_json::to_string(&connect_message);
    match serialized_connect_message {
        Ok(message) => {
            let _ = socket.write_message(Message::Text(message));
            socket
        }
        Err(e) => panic!("Unable to create connect message: {}", e),
    }
}

fn send_pixels_per_point(socket: &mut WebSocket<MaybeTlsStream<TcpStream>>, pixels_per_point: f32) {
    let pixels_per_point_message =
        eframe::RemoteRenderingMessageType::PixelsPerPoint(pixels_per_point);
    let serialized_pixels_per_point_message = serde_json::to_string(&pixels_per_point_message);
    match serialized_pixels_per_point_message {
        Ok(message) => {
            let _ = socket.write_message(Message::Text(message));
        }
        Err(e) => panic!("Unable to create pixels_per_point message: {}", e),
    }
}

fn send_raw_input(socket: &mut WebSocket<MaybeTlsStream<TcpStream>>, raw_input: egui::RawInput) {
    let raw_input_message = eframe::RemoteRenderingMessageType::RawInput(raw_input);
    let serialized_raw_input_message = serde_json::to_string(&raw_input_message);
    match serialized_raw_input_message {
        Ok(message) => {
            let _ = socket.write_message(Message::Text(message));
        }
        Err(e) => panic!("Unable to create raw input message: {}", e),
    }
}

struct MyApp {
    socket: WebSocket<MaybeTlsStream<TcpStream>>,
    pixels_per_point: f32,
    full_output: egui::FullOutput,
}

impl Default for MyApp {
    fn default() -> Self {
        // Set to appropriate value
        let pixels_per_point = 2.0;
        let socket = connect_to_server(pixels_per_point);
        Self {
            socket,
            pixels_per_point,
            full_output: egui::FullOutput::default(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {}
    fn update_remote(
        &mut self,
        raw_input: egui::RawInput,
        frame: &eframe::Frame,
    ) -> (egui::output::FullOutput, f32) {
        // Send pixels_per_point value if it has changed
        if let Some(ppp) = frame.info().native_pixels_per_point {
            if ppp != self.pixels_per_point {
                send_pixels_per_point(&mut self.socket, ppp);
                self.pixels_per_point = ppp;
            }
        }
        // Send captured input
        if !raw_input.events.is_empty() {
            send_raw_input(&mut self.socket, raw_input);
        } else {
            send_raw_input(&mut self.socket, egui::RawInput::default());
        };
        let received_message = self
            .socket
            .read_message()
            .expect("Error reading message from server");
        // Filter out ping messages
        if let tungstenite::Message::Ping(_) = received_message {
            return (self.full_output.clone(), self.pixels_per_point);
        }

        let full_output_diff = received_message.into_text();

        if let Ok(diff) = full_output_diff {
            let mut deserializer = serde_json::Deserializer::from_str(&diff);
            let _ = Apply::apply(&mut deserializer, &mut self.full_output);
            //let deserialized: egui::FullOutput = serde_json::from_str(&diff).unwrap();
            //self.full_output = deserialized;
        }

        (self.full_output.clone(), self.pixels_per_point)
    }
}
