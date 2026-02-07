//! Demo app for egui

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example
#![allow(clippy::allow_attributes, clippy::never_loop)]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc; // Much faster allocator, can give 20% speedups: https://github.com/emilk/egui/pull/7029

// When compiling natively:
fn main() {
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--profile" => {
                #[cfg(feature = "puffin")]
                start_puffin_server();

                #[cfg(not(feature = "puffin"))]
                panic!(
                    "Unknown argument: {arg} - you need to enable the 'puffin' feature to use this."
                );
            }

            _ => {
                panic!("Unknown argument: {arg}");
            }
        }
    }

    {
        // Silence wgpu log spam (https://github.com/gfx-rs/wgpu/issues/3206)
        let mut rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                "debug".to_owned()
            } else {
                "info".to_owned()
            }
        });
        for loud_crate in ["naga", "wgpu_core", "wgpu_hal"] {
            if !rust_log.contains(&format!("{loud_crate}=")) {
                rust_log += &format!(",{loud_crate}=warn");
            }
        }

        // SAFETY: we call this from the main thread without any other threads running.
        #[expect(unsafe_code)]
        unsafe {
            std::env::set_var("RUST_LOG", rust_log);
        }
    }

    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 1024.0])
            .with_drag_and_drop(true),

        #[cfg(feature = "wgpu")]
        renderer: eframe::Renderer::Wgpu,

        ..Default::default()
    };

    let result = eframe::run_native(
        "egui demo app",
        options,
        Box::new(|cc| Ok(Box::new(egui_demo_app::WrapApp::new(cc)))),
    );

    match result {
        Ok(()) => {}
        Err(err) => {
            // This produces a nicer error message than returning the `Result`:
            print_error_and_exit(&err);
        }
    }
}

fn print_error_and_exit(err: &eframe::Error) -> ! {
    #![expect(clippy::print_stderr)]
    #![expect(clippy::exit)]

    eprintln!("Error: {err}");
    std::process::exit(1)
}

#[cfg(feature = "puffin")]
fn start_puffin_server() {
    puffin::set_scopes_on(true); // tell puffin to collect data

    match puffin_http::Server::new("127.0.0.1:8585") {
        Ok(puffin_server) => {
            log::info!("Run:  cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585");

            std::process::Command::new("puffin_viewer")
                .arg("--url")
                .arg("127.0.0.1:8585")
                .spawn()
                .ok();

            // We can store the server if we want, but in this case we just want
            // it to keep running. Dropping it closes the server, so let's not drop it!
            #[expect(clippy::mem_forget)]
            std::mem::forget(puffin_server);
        }
        Err(err) => {
            log::error!("Failed to start puffin server: {err}");
        }
    }
}
