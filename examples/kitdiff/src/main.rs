#[cfg(not(target_arch = "wasm32"))]
mod cli;

#[cfg(not(target_arch = "wasm32"))]
use eframe::NativeOptions;
use kitdiff::DiffSource;
use kitdiff::app::App;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    use clap::Parser;
    let mode = cli::Cli::parse();

    let source = mode
        .command
        .map(|c| c.to_source())
        .unwrap_or(DiffSource::Files);

    eframe::run_native(
        "kitdiff",
        NativeOptions::default(),
        Box::new(move |cc| Ok(Box::new(App::new(cc, Some(source))))),
    )
}

#[cfg(target_arch = "wasm32")]
fn parse_url_query_params() -> Option<DiffSource> {
    use kitdiff::github_auth::parse_github_artifact_url;

    if let Some(window) = web_sys::window() {
        if let Ok(search) = window.location().search() {
            let search = search.strip_prefix('?').unwrap_or(&search);

            // Parse query parameters
            for param in search.split('&') {
                if let Some((key, value)) = param.split_once('=') {
                    if key == "url" {
                        // URL decode the value
                        let decoded_url = js_sys::decode_uri_component(value)
                            .ok()?
                            .as_string()?;

                        // Try to parse as GitHub artifact URL
                        if let Some((owner, repo, artifact_id)) = parse_github_artifact_url(&decoded_url) {
                            return Some(DiffSource::GHArtifact { owner, repo, artifact_id });
                        }

                        // Try to parse as direct zip/tar.gz URL
                        if decoded_url.ends_with(".zip") {
                            return Some(DiffSource::Zip(kitdiff::PathOrBlob::Url(decoded_url, None)));
                        }
                        if decoded_url.ends_with(".tar.gz") || decoded_url.ends_with(".tgz") {
                            return Some(DiffSource::TarGz(kitdiff::PathOrBlob::Url(decoded_url, None)));
                        }
                    }
                }
            }
        }
    }
    None
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;
    use web_sys::HtmlCanvasElement;

    let web_options = eframe::WebOptions::default();
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document
            .get_element_by_id("the_canvas_id")
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()
            .unwrap();

        // Parse URL query parameters for DiffSource
        let diff_source = parse_url_query_params();

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(move |cc| Ok(Box::new(App::new(cc, diff_source)))),
            )
            .await;

        // Remove the loading text and un-hide the canvas
        let loading_text = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("loading_text"));
        if let Some(loading_text) = loading_text {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
