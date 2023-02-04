#![allow(special_module_name)]

mod main;

// Entry point for wasm
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let web_options = eframe::WebOptions::default();
    eframe::start_web(
        "my",
        web_options,
        Box::new(|cc| Box::new(main::MyApp::new(cc))),
    )?;
    Ok(())
}
