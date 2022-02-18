//! A good way of displaying an SVG image in egui.
//!
//! Requires the dependencies `resvg`, `tiny-skia`, `usvg`
use eframe::{egui, epi};

/// Load an SVG and rasterize it into an egui image.
fn load_svg_data(svg_data: &[u8]) -> Result<egui::ColorImage, String> {
    let mut opt = usvg::Options::default();
    opt.fontdb.load_system_fonts();

    let rtree = usvg::Tree::from_data(svg_data, &opt.to_ref()).map_err(|err| err.to_string())?;

    let pixmap_size = rtree.svg_node().size.to_screen_size();
    let [w, h] = [pixmap_size.width(), pixmap_size.height()];

    let mut pixmap = tiny_skia::Pixmap::new(w, h)
        .ok_or_else(|| format!("Failed to create SVG Pixmap of size {}x{}", w, h))?;

    resvg::render(
        &rtree,
        usvg::FitTo::Original,
        tiny_skia::Transform::default(),
        pixmap.as_mut(),
    )
    .ok_or_else(|| "Failed to render SVG".to_owned())?;

    let image = egui::ColorImage::from_rgba_unmultiplied(
        [pixmap.width() as _, pixmap.height() as _],
        pixmap.data(),
    );

    Ok(image)
}

// ----------------------------------------------------------------------------

/// An SVG image to be shown in egui
struct SvgImage {
    image: egui::ColorImage,
    texture: Option<egui::TextureHandle>,
}

impl SvgImage {
    /// Pass itn the bytes of an SVG that you've loaded from disk
    pub fn from_svg_data(bytes: &[u8]) -> Result<Self, String> {
        Ok(Self {
            image: load_svg_data(bytes)?,
            texture: None,
        })
    }

    pub fn show_max_size(&mut self, ui: &mut egui::Ui, max_size: egui::Vec2) -> egui::Response {
        let mut desired_size = egui::vec2(self.image.width() as _, self.image.height() as _);
        desired_size *= (max_size.x / desired_size.x).min(1.0);
        desired_size *= (max_size.y / desired_size.y).min(1.0);
        self.show_size(ui, desired_size)
    }

    pub fn show_size(&mut self, ui: &mut egui::Ui, desired_size: egui::Vec2) -> egui::Response {
        // We need to convert the SVG to a texture to display it:
        // Future improvement: tell backend to do mip-mapping of the image to
        // make it look smoother when downsized.
        let svg_texture = self
            .texture
            .get_or_insert_with(|| ui.ctx().load_texture("svg", self.image.clone()));
        ui.image(svg_texture, desired_size)
    }
}

// ----------------------------------------------------------------------------

struct MyApp {
    svg_image: SvgImage,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            svg_image: SvgImage::from_svg_data(include_bytes!("rustacean-flat-happy.svg")).unwrap(),
        }
    }
}

impl epi::App for MyApp {
    fn name(&self) -> &str {
        "svg example"
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("SVG example");
            ui.label("The SVG is rasterized and displayed as a texture.");

            ui.separator();

            let max_size = ui.available_size();
            self.svg_image.show_max_size(ui, max_size);
        });
    }
}

fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1000.0, 700.0)),
        ..Default::default()
    };
    eframe::run_native(Box::new(MyApp::default()), options);
}
