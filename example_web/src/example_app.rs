use egui_web::fetch::Response;
use std::sync::mpsc::Receiver;

struct Resource {
    /// HTTP response
    response: Response,

    /// If set, the response was an image.
    image: Option<Image>,

    /// If set, the response was text with some supported syntax highlighting (e.g. ".rs" or ".md").
    colored_text: Option<ColoredText>,
}

impl Resource {
    fn from_response(response: Response) -> Self {
        let image = if response.header_content_type.starts_with("image/") {
            Image::decode(&response.bytes)
        } else {
            None
        };

        let colored_text = syntax_highlighting(&response);

        Self {
            response,
            image,
            colored_text,
        }
    }
}

pub struct ExampleApp {
    url: String,
    in_progress: Option<Receiver<Result<Response, String>>>,
    result: Option<Result<Resource, String>>,
    tex_mngr: TexMngr,
}

impl Default for ExampleApp {
    fn default() -> Self {
        Self {
            url: "https://raw.githubusercontent.com/emilk/egui/master/README.md".to_owned(),
            in_progress: Default::default(),
            result: Default::default(),
            tex_mngr: Default::default(),
        }
    }
}

impl egui::app::App for ExampleApp {
    fn name(&self) -> &str {
        "Egui Fetch Example"
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn ui(&mut self, ctx: &egui::CtxRef, integration_context: &mut egui::app::IntegrationContext) {
        if let Some(receiver) = &mut self.in_progress {
            // Are we there yet?
            if let Ok(result) = receiver.try_recv() {
                self.in_progress = None;
                self.result = Some(result.map(Resource::from_response));
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Egui Fetch Example");
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/egui/blob/master/",
                "(source code)"
            ));

            if let Some(url) = ui_url(ui, &mut self.url) {
                let repaint_signal = integration_context.repaint_signal.clone();
                let (sender, receiver) = std::sync::mpsc::channel();
                self.in_progress = Some(receiver);
                egui_web::spawn_future(async move {
                    sender.send(egui_web::fetch::get(&url).await).ok();
                    repaint_signal.request_repaint();
                });
            }

            ui.separator();

            if self.in_progress.is_some() {
                ui.label("Please wait...");
            } else if let Some(result) = &self.result {
                match result {
                    Ok(resource) => {
                        ui_resouce(ui, integration_context, &mut self.tex_mngr, resource);
                    }
                    Err(error) => {
                        // This should only happen if the fetch API isn't available or something similar.
                        ui.add(egui::Label::new(error).text_color(egui::color::RED));
                    }
                }
            }
        });
    }
}

fn ui_url(ui: &mut egui::Ui, url: &mut String) -> Option<String> {
    let mut trigger_fetch = false;

    ui.horizontal(|ui| {
        ui.label("URL:");
        trigger_fetch |= ui.text_edit_singleline(url).lost_kb_focus;
        trigger_fetch |= ui.button("GET").clicked;
    });

    ui.label("HINT: paste the url of this page into the field above!");

    ui.horizontal(|ui| {
        if ui.button("Source code for this example").clicked {
            *url = format!(
                "https://raw.githubusercontent.com/emilk/egui/master/{}",
                file!()
            );
            trigger_fetch = true;
        }
        if ui.button("Random image").clicked {
            let seed = ui.input().time;
            let width = 640;
            let height = 480;
            *url = format!("https://picsum.photos/seed/{}/{}/{}", seed, width, height);
            trigger_fetch = true;
        }
    });

    if trigger_fetch {
        Some(url.clone())
    } else {
        None
    }
}

fn ui_resouce(
    ui: &mut egui::Ui,
    integration_context: &mut egui::app::IntegrationContext,
    tex_mngr: &mut TexMngr,
    resource: &Resource,
) {
    let Resource {
        response,
        image,
        colored_text,
    } = resource;

    ui.monospace(format!("url:          {}", response.url));
    ui.monospace(format!(
        "status:       {} ({})",
        response.status, response.status_text
    ));
    ui.monospace(format!("Content-Type: {}", response.header_content_type));
    ui.monospace(format!(
        "Size:         {:.1} kB",
        response.bytes.len() as f32 / 1000.0
    ));

    if let Some(text) = &response.text {
        let tooltip = "Click to copy the response body";
        if ui.button("📋").on_hover_text(tooltip).clicked {
            ui.output().copied_text = text.clone();
        }
    }

    ui.separator();

    egui::ScrollArea::auto_sized().show(ui, |ui| {
        if let Some(image) = image {
            if let Some(texture_id) = tex_mngr.texture(integration_context, &response.url, &image) {
                let size = egui::Vec2::new(image.size.0 as f32, image.size.1 as f32);
                ui.image(texture_id, size);
            }
        } else if let Some(colored_text) = colored_text {
            colored_text.ui(ui);
        } else if let Some(text) = &response.text {
            ui.monospace(text);
        } else {
            ui.monospace("[binary]");
        }
    });
}

// ----------------------------------------------------------------------------
// Syntax highlighting:

fn syntax_highlighting(response: &Response) -> Option<ColoredText> {
    let text = response.text.as_ref()?;
    let extension_and_rest: Vec<&str> = response.url.rsplitn(2, '.').collect();
    let extension = extension_and_rest.get(0)?;
    ColoredText::text_with_extension(text, extension)
}

/// Lines of text fragments
struct ColoredText(Vec<Vec<(syntect::highlighting::Style, String)>>);

impl ColoredText {
    /// e.g. `text_with_extension("fn foo() {}", "rs")
    pub fn text_with_extension(text: &str, extension: &str) -> Option<ColoredText> {
        use syntect::easy::HighlightLines;
        use syntect::highlighting::ThemeSet;
        use syntect::parsing::SyntaxSet;
        use syntect::util::LinesWithEndings;

        let ps = SyntaxSet::load_defaults_newlines(); // should be cached and reused
        let ts = ThemeSet::load_defaults(); // should be cached and reused

        let syntax = ps.find_syntax_by_extension(extension)?;

        let mut h = HighlightLines::new(syntax, &ts.themes["base16-mocha.dark"]);

        let lines = LinesWithEndings::from(&text)
            .map(|line| {
                h.highlight(line, &ps)
                    .into_iter()
                    .map(|(style, range)| (style, range.trim_end_matches('\n').to_owned()))
                    .collect()
            })
            .collect();

        Some(ColoredText(lines))
    }

    pub fn ui(&self, ui: &mut egui::Ui) {
        for line in &self.0 {
            ui.horizontal_wrapped_for_text(egui::TextStyle::Monospace, |ui| {
                ui.style_mut().spacing.item_spacing.x = 0.0;
                for (style, range) in line {
                    let fg = style.foreground;
                    let text_color = egui::Srgba::from_rgb(fg.r, fg.g, fg.b);
                    ui.add(egui::Label::new(range).monospace().text_color(text_color));
                }
            });
        }
    }
}

// ----------------------------------------------------------------------------
// Texture/image handling is very manual at the moment.

/// Immediate mode texture manager that supports at most one texture at the time :)
#[derive(Default)]
struct TexMngr {
    loaded_url: String,
    texture_id: Option<egui::TextureId>,
}

impl TexMngr {
    fn texture(
        &mut self,
        integration_context: &mut egui::app::IntegrationContext,
        url: &str,
        image: &Image,
    ) -> Option<egui::TextureId> {
        let tex_allocator = integration_context.tex_allocator.as_mut()?;
        let texture_id = self.texture_id.unwrap_or_else(|| tex_allocator.alloc());
        self.texture_id = Some(texture_id);
        if self.loaded_url != url {
            self.loaded_url = url.to_owned();
            tex_allocator.set_srgba_premultiplied(texture_id, image.size, &image.pixels);
        }
        Some(texture_id)
    }
}

struct Image {
    size: (usize, usize),
    pixels: Vec<egui::Srgba>,
}

impl Image {
    fn decode(bytes: &[u8]) -> Option<Image> {
        use image::GenericImageView;
        let image = image::load_from_memory(&bytes).ok()?;
        let image_buffer = image.to_rgba8();
        let size = (image.width() as usize, image.height() as usize);
        let pixels = image_buffer.into_vec();
        assert_eq!(size.0 * size.1 * 4, pixels.len());
        let pixels = pixels
            .chunks(4)
            .map(|p| egui::Srgba::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect();

        Some(Image { size, pixels })
    }
}
