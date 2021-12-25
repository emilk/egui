use std::sync::mpsc::Receiver;

struct Resource {
    /// HTTP response
    response: ehttp::Response,

    text: Option<String>,

    /// If set, the response was an image.
    image: Option<epi::Image>,

    /// If set, the response was text with some supported syntax highlighting (e.g. ".rs" or ".md").
    colored_text: Option<ColoredText>,
}

impl Resource {
    fn from_response(ctx: &egui::Context, response: ehttp::Response) -> Self {
        let content_type = response.content_type().unwrap_or_default();
        let image = if content_type.starts_with("image/") {
            decode_image(&response.bytes)
        } else {
            None
        };

        let text = response.text();

        let colored_text = text
            .as_ref()
            .and_then(|text| syntax_highlighting(ctx, &response, text));

        Self {
            response,
            text,
            image,
            colored_text,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct HttpApp {
    url: String,

    #[cfg_attr(feature = "serde", serde(skip))]
    in_progress: Option<Receiver<Result<ehttp::Response, String>>>,

    #[cfg_attr(feature = "serde", serde(skip))]
    result: Option<Result<Resource, String>>,

    #[cfg_attr(feature = "serde", serde(skip))]
    tex_mngr: TexMngr,
}

impl Default for HttpApp {
    fn default() -> Self {
        Self {
            url: "https://raw.githubusercontent.com/emilk/egui/master/README.md".to_owned(),
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

    fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
        if let Some(receiver) = &mut self.in_progress {
            // Are we there yet?
            if let Ok(result) = receiver.try_recv() {
                self.in_progress = None;
                self.result = Some(result.map(|response| Resource::from_response(ctx, response)));
            }
        }

        egui::TopBottomPanel::bottom("http_bottom").show(ctx, |ui| {
            let layout = egui::Layout::top_down(egui::Align::Center).with_main_justify(true);
            ui.allocate_ui_with_layout(ui.available_size(), layout, |ui| {
                ui.add(crate::__egui_github_link_file!())
            })
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let trigger_fetch = ui_url(ui, frame, &mut self.url);

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("HTTP requests made using ");
                ui.hyperlink_to("ehttp", "https://www.github.com/emilk/ehttp");
                ui.label(".");
            });

            if trigger_fetch {
                let request = ehttp::Request::get(&self.url);
                let frame = frame.clone();
                let (sender, receiver) = std::sync::mpsc::channel();
                self.in_progress = Some(receiver);

                ehttp::fetch(request, move |response| {
                    sender.send(response).ok();
                    frame.request_repaint();
                });
            }

            ui.separator();

            if self.in_progress.is_some() {
                ui.label("Please wait…");
            } else if let Some(result) = &self.result {
                match result {
                    Ok(resource) => {
                        ui_resource(ui, frame, &mut self.tex_mngr, resource);
                    }
                    Err(error) => {
                        // This should only happen if the fetch API isn't available or something similar.
                        ui.colored_label(
                            egui::Color32::RED,
                            if error.is_empty() { "Error" } else { error },
                        );
                    }
                }
            }
        });
    }
}

fn ui_url(ui: &mut egui::Ui, frame: &epi::Frame, url: &mut String) -> bool {
    let mut trigger_fetch = false;

    ui.horizontal(|ui| {
        ui.label("URL:");
        trigger_fetch |= ui
            .add(egui::TextEdit::singleline(url).desired_width(f32::INFINITY))
            .lost_focus();
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
            trigger_fetch = true;
        }
        if ui.button("Random image").clicked() {
            let seed = ui.input().time;
            let side = 640;
            *url = format!("https://picsum.photos/seed/{}/{}", seed, side);
            trigger_fetch = true;
        }
    });

    trigger_fetch
}

fn ui_resource(ui: &mut egui::Ui, frame: &epi::Frame, tex_mngr: &mut TexMngr, resource: &Resource) {
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

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
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
                ui.separator();
            }

            if let Some(image) = image {
                if let Some(texture_id) = tex_mngr.texture(frame, &response.url, image) {
                    let mut size = egui::Vec2::new(image.size[0] as f32, image.size[1] as f32);
                    size *= (ui.available_width() / size.x).min(1.0);
                    ui.image(texture_id, size);
                }
            } else if let Some(colored_text) = colored_text {
                colored_text.ui(ui);
            } else if let Some(text) = &text {
                selectable_text(ui, text);
            } else {
                ui.monospace("[binary]");
            }
        });
}

fn selectable_text(ui: &mut egui::Ui, mut text: &str) {
    ui.add(
        egui::TextEdit::multiline(&mut text)
            .desired_width(f32::INFINITY)
            .text_style(egui::TextStyle::Monospace),
    );
}

// ----------------------------------------------------------------------------
// Syntax highlighting:

#[cfg(feature = "syntect")]
fn syntax_highlighting(
    ctx: &egui::Context,
    response: &ehttp::Response,
    text: &str,
) -> Option<ColoredText> {
    let extension_and_rest: Vec<&str> = response.url.rsplitn(2, '.').collect();
    let extension = extension_and_rest.get(0)?;
    let theme = crate::syntax_highlighting::CodeTheme::from_style(&ctx.style());
    Some(ColoredText(crate::syntax_highlighting::highlight(
        ctx, &theme, text, extension,
    )))
}

#[cfg(not(feature = "syntect"))]
fn syntax_highlighting(_ctx: &egui::Context, _: &ehttp::Response, _: &str) -> Option<ColoredText> {
    None
}

struct ColoredText(egui::text::LayoutJob);

impl ColoredText {
    pub fn ui(&self, ui: &mut egui::Ui) {
        if true {
            // Selectable text:
            let mut layouter = |ui: &egui::Ui, _string: &str, wrap_width: f32| {
                let mut layout_job = self.0.clone();
                layout_job.wrap_width = wrap_width;
                ui.fonts().layout_job(layout_job)
            };

            let mut text = self.0.text.as_str();
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .text_style(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .layouter(&mut layouter),
            );
        } else {
            let mut job = self.0.clone();
            job.wrap_width = ui.available_width();
            let galley = ui.fonts().layout_job(job);
            let (response, painter) = ui.allocate_painter(galley.size(), egui::Sense::hover());
            painter.add(egui::Shape::galley(response.rect.min, galley));
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
        frame: &epi::Frame,
        url: &str,
        image: &epi::Image,
    ) -> Option<egui::TextureId> {
        if self.loaded_url != url {
            if let Some(texture_id) = self.texture_id.take() {
                frame.free_texture(texture_id);
            }

            self.texture_id = Some(frame.alloc_texture(image.clone()));
            self.loaded_url = url.to_owned();
        }
        self.texture_id
    }
}

fn decode_image(bytes: &[u8]) -> Option<epi::Image> {
    use image::GenericImageView;
    let image = image::load_from_memory(bytes).ok()?;
    let image_buffer = image.to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image_buffer.into_vec();
    Some(epi::Image::from_rgba_unmultiplied(size, &pixels))
}
