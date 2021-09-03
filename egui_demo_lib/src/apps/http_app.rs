use std::sync::mpsc::Receiver;

struct Resource {
    /// HTTP response
    response: ehttp::Response,

    text: Option<String>,

    /// If set, the response was an image.
    image: Option<Image>,

    /// If set, the response was text with some supported syntax highlighting (e.g. ".rs" or ".md").
    colored_text: Option<ColoredText>,
}

impl Resource {
    fn from_response(response: ehttp::Response) -> Self {
        let content_type = response.content_type().unwrap_or_default();
        let image = if content_type.starts_with("image/") {
            Image::decode(&response.bytes)
        } else {
            None
        };

        let text = response.text();

        let colored_text = text
            .as_ref()
            .and_then(|text| syntax_highlighting(&response, text));

        Self {
            response,
            text,
            image,
            colored_text,
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
enum Method {
    Get,
    Post,
}

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct HttpApp {
    url: String,

    method: Method,

    request_body: String,

    #[cfg_attr(feature = "persistence", serde(skip))]
    in_progress: Option<Receiver<Result<ehttp::Response, String>>>,

    #[cfg_attr(feature = "persistence", serde(skip))]
    result: Option<Result<Resource, String>>,

    #[cfg_attr(feature = "persistence", serde(skip))]
    tex_mngr: TexMngr,
}

impl Default for HttpApp {
    fn default() -> Self {
        Self {
            url: "https://raw.githubusercontent.com/emilk/egui/master/README.md".to_owned(),
            method: Method::Get,
            request_body: r#"["posting some json", { "more_json" : true }]"#.to_owned(),
            in_progress: Default::default(),
            result: Default::default(),
            tex_mngr: Default::default(),
        }
    }
}

impl epi::App for HttpApp {
    fn name(&self) -> &str {
        "⬇ HTTP"
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        if let Some(receiver) = &mut self.in_progress {
            // Are we there yet?
            if let Ok(result) = receiver.try_recv() {
                self.in_progress = None;
                self.result = Some(result.map(Resource::from_response));
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("HTTP Fetch Example");
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("HTTP requests made using ");
                ui.hyperlink_to("ehttp", "https://www.github.com/emilk/ehttp");
                ui.label(".");
            });
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/egui/blob/master/",
                "(demo source code)"
            ));

            if let Some(request) = ui_url(
                ui,
                frame,
                &mut self.url,
                &mut self.method,
                &mut self.request_body,
            ) {
                let repaint_signal = frame.repaint_signal();
                let (sender, receiver) = std::sync::mpsc::channel();
                self.in_progress = Some(receiver);

                ehttp::fetch(request, move |response| {
                    sender.send(response).ok();
                    repaint_signal.request_repaint();
                });
            }

            ui.separator();

            if self.in_progress.is_some() {
                ui.label("Please wait...");
            } else if let Some(result) = &self.result {
                match result {
                    Ok(resource) => {
                        ui_resource(ui, frame, &mut self.tex_mngr, resource);
                    }
                    Err(error) => {
                        // This should only happen if the fetch API isn't available or something similar.
                        ui.add(
                            egui::Label::new(if error.is_empty() { "Error" } else { error })
                                .text_color(egui::Color32::RED),
                        );
                    }
                }
            }
        });
    }
}

fn ui_url(
    ui: &mut egui::Ui,
    frame: &mut epi::Frame<'_>,
    url: &mut String,
    method: &mut Method,
    request_body: &mut String,
) -> Option<ehttp::Request> {
    let mut trigger_fetch = false;

    egui::Grid::new("request_params").show(ui, |ui| {
        ui.label("URL:");
        ui.horizontal(|ui| {
            trigger_fetch |= ui.text_edit_singleline(url).lost_focus();
            egui::ComboBox::from_id_source("method")
                .selected_text(format!("{:?}", method))
                .width(60.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(method, Method::Get, "GET");
                    ui.selectable_value(method, Method::Post, "POST");
                });
            trigger_fetch |= ui.button("▶").clicked();
        });
        ui.end_row();
        if *method == Method::Post {
            ui.label("Body:");
            ui.add(
                egui::TextEdit::multiline(request_body)
                    .code_editor()
                    .desired_rows(1),
            );
            ui.end_row();
        }
    });

    if frame.is_web() {
        ui.label("HINT: paste the url of this page into the field above!");
    }

    ui.horizontal(|ui| {
        if ui.button("Source code for this example").clicked() {
            *url = format!(
                "https://raw.githubusercontent.com/emilk/egui/master/{}",
                file!()
            );
            *method = Method::Get;
            trigger_fetch = true;
        }
        if ui.button("Random image").clicked() {
            let seed = ui.input().time;
            let width = 640;
            let height = 480;
            *url = format!("https://picsum.photos/seed/{}/{}/{}", seed, width, height);
            *method = Method::Get;
            trigger_fetch = true;
        }
        if ui.button("Post to httpbin.org").clicked() {
            *url = "https://httpbin.org/post".to_owned();
            *method = Method::Post;
            trigger_fetch = true;
        }
    });

    if trigger_fetch {
        Some(match *method {
            Method::Get => ehttp::Request::get(url),
            Method::Post => ehttp::Request::post(url, request_body),
        })
    } else {
        None
    }
}

fn ui_resource(
    ui: &mut egui::Ui,
    frame: &mut epi::Frame<'_>,
    tex_mngr: &mut TexMngr,
    resource: &Resource,
) {
    let Resource {
        response,
        text,
        image,
        colored_text,
    } = resource;

    ui.monospace(format!("url:          {}", response.url));
    ui.monospace(format!(
        "status:       {} ({})",
        response.status, response.status_text
    ));
    ui.monospace(format!(
        "content-type: {}",
        response.content_type().unwrap_or_default()
    ));
    ui.monospace(format!(
        "size:         {:.1} kB",
        response.bytes.len() as f32 / 1000.0
    ));

    ui.separator();

    egui::CollapsingHeader::new("Response headers")
        .default_open(false)
        .show(ui, |ui| {
            egui::Grid::new("response_headers")
                .spacing(egui::vec2(ui.spacing().item_spacing.x * 2.0, 0.0))
                .show(ui, |ui| {
                    for header in &response.headers {
                        ui.label(header.0);
                        ui.label(header.1);
                        ui.end_row();
                    }
                })
        });

    ui.separator();

    if let Some(text) = &text {
        let tooltip = "Click to copy the response body";
        if ui.button("📋").on_hover_text(tooltip).clicked() {
            ui.output().copied_text = text.clone();
        }
    }

    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        if let Some(image) = image {
            if let Some(texture_id) = tex_mngr.texture(frame, &response.url, image) {
                let size = egui::Vec2::new(image.size.0 as f32, image.size.1 as f32);
                ui.image(texture_id, size);
            }
        } else if let Some(colored_text) = colored_text {
            colored_text.ui(ui);
        } else if let Some(text) = &text {
            ui.monospace(text);
        } else {
            ui.monospace("[binary]");
        }
    });
}

// ----------------------------------------------------------------------------
// Syntax highlighting:

#[cfg(feature = "syntect")]
fn syntax_highlighting(response: &ehttp::Response, text: &str) -> Option<ColoredText> {
    let extension_and_rest: Vec<&str> = response.url.rsplitn(2, '.').collect();
    let extension = extension_and_rest.get(0)?;
    ColoredText::text_with_extension(text, extension)
}

/// Lines of text fragments
#[cfg(feature = "syntect")]
struct ColoredText(egui::text::LayoutJob);

#[cfg(feature = "syntect")]
impl ColoredText {
    /// e.g. `text_with_extension("fn foo() {}", "rs")`
    pub fn text_with_extension(text: &str, extension: &str) -> Option<ColoredText> {
        use syntect::easy::HighlightLines;
        use syntect::highlighting::FontStyle;
        use syntect::highlighting::ThemeSet;
        use syntect::parsing::SyntaxSet;
        use syntect::util::LinesWithEndings;

        let ps = SyntaxSet::load_defaults_newlines(); // should be cached and reused
        let ts = ThemeSet::load_defaults(); // should be cached and reused

        let syntax = ps.find_syntax_by_extension(extension)?;

        let dark_mode = true;
        let theme = if dark_mode {
            "base16-mocha.dark"
        } else {
            "base16-ocean.light"
        };
        let mut h = HighlightLines::new(syntax, &ts.themes[theme]);

        use egui::text::{LayoutJob, LayoutSection, TextFormat};

        let mut job = LayoutJob {
            text: text.into(),
            ..Default::default()
        };

        for line in LinesWithEndings::from(text) {
            for (style, range) in h.highlight(line, &ps) {
                let fg = style.foreground;
                let text_color = egui::Color32::from_rgb(fg.r, fg.g, fg.b);
                let italics = style.font_style.contains(FontStyle::ITALIC);
                let underline = style.font_style.contains(FontStyle::ITALIC);
                let underline = if underline {
                    egui::Stroke::new(1.0, text_color)
                } else {
                    egui::Stroke::none()
                };
                job.sections.push(LayoutSection {
                    leading_space: 0.0,
                    byte_range: as_byte_range(text, range),
                    format: TextFormat {
                        style: egui::TextStyle::Monospace,
                        color: text_color,
                        italics,
                        underline,
                        ..Default::default()
                    },
                });
            }
        }

        Some(ColoredText(job))
    }

    pub fn ui(&self, ui: &mut egui::Ui) {
        let mut job = self.0.clone();
        job.wrap_width = ui.available_width();
        let galley = ui.fonts().layout_job(job);
        let (response, painter) = ui.allocate_painter(galley.size, egui::Sense::hover());
        painter.add(egui::Shape::galley(response.rect.min, galley));
    }
}

#[cfg(feature = "syntect")]
fn as_byte_range(whole: &str, range: &str) -> std::ops::Range<usize> {
    let whole_start = whole.as_ptr() as usize;
    let range_start = range.as_ptr() as usize;
    assert!(whole_start <= range_start);
    assert!(range_start + range.len() <= whole_start + whole.len());
    let offset = range_start - whole_start;
    offset..(offset + range.len())
}

#[cfg(not(feature = "syntect"))]
fn syntax_highlighting(_: &ehttp::Response, _: &str) -> Option<ColoredText> {
    None
}
#[cfg(not(feature = "syntect"))]
struct ColoredText();
#[cfg(not(feature = "syntect"))]
impl ColoredText {
    pub fn ui(&self, _ui: &mut egui::Ui) {}
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
        frame: &mut epi::Frame<'_>,
        url: &str,
        image: &Image,
    ) -> Option<egui::TextureId> {
        if self.loaded_url != url {
            if let Some(texture_id) = self.texture_id.take() {
                frame.tex_allocator().free(texture_id);
            }

            self.texture_id = Some(
                frame
                    .tex_allocator()
                    .alloc_srgba_premultiplied(image.size, &image.pixels),
            );
            self.loaded_url = url.to_owned();
        }
        self.texture_id
    }
}

struct Image {
    size: (usize, usize),
    pixels: Vec<egui::Color32>,
}

impl Image {
    fn decode(bytes: &[u8]) -> Option<Image> {
        use image::GenericImageView;
        let image = image::load_from_memory(bytes).ok()?;
        let image_buffer = image.to_rgba8();
        let size = (image.width() as usize, image.height() as usize);
        let pixels = image_buffer.into_vec();
        assert_eq!(size.0 * size.1 * 4, pixels.len());
        let pixels = pixels
            .chunks(4)
            .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect();

        Some(Image { size, pixels })
    }
}
