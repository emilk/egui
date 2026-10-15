use std::collections::BTreeMap;

struct GlyphInfo {
    name: String,

    // What fonts it is available in
    fonts: Vec<String>,
}

pub struct FontBook {
    filter: String,
    font_id: egui::FontId,
    available_glyphs: BTreeMap<egui::FontFamily, BTreeMap<char, GlyphInfo>>,
    #[cfg(target_arch = "wasm32")]
    include_missing_emoji: bool,
}

impl Default for FontBook {
    fn default() -> Self {
        Self {
            filter: Default::default(),
            font_id: egui::FontId::proportional(18.0),
            available_glyphs: Default::default(),
            #[cfg(target_arch = "wasm32")]
            include_missing_emoji: false,
        }
    }
}

impl crate::Demo for FontBook {
    fn name(&self) -> &'static str {
        "🔤 Font Book"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for FontBook {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("On web, the browser draws missing glyphs instead. ");
            ui.label("On native, eframe falls back to looking for a matching system font. ");
            ui.label("Bundle fonts with ");
            ui.add(egui::Hyperlink::from_label_and_url(
                egui::RichText::new("Context::add_font").text_style(egui::TextStyle::Monospace),
                "https://docs.rs/egui/latest/egui/struct.Context.html#method.add_font",
            ));
            ui.label(".");
        });

        ui.separator();

        egui::introspection::font_id_ui(ui, &mut self.font_id);

        let font_id = self.font_id.clone();

        ui.horizontal(|ui| {
            let label = ui.label("Filter:");
            ui.add(egui::TextEdit::singleline(&mut self.filter).desired_width(120.0))
                .labelled_by(label.id);
            self.filter = self.filter.to_lowercase();
            if ui.button("ｘ").clicked() {
                self.filter.clear();
            }
        });

        #[cfg(target_arch = "wasm32")]
        if ui
            .checkbox(
                &mut self.include_missing_emoji,
                "Show emoji missing from selected font",
            )
            .on_hover_text(
                "Include emoji candidates even when the selected font has no glyph for them.",
            )
            .changed()
        {
            self.available_glyphs.clear();
        }

        let include_missing_emoji = {
            #[cfg(target_arch = "wasm32")]
            {
                self.include_missing_emoji
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                false
            }
        };
        let available_glyphs = self
            .available_glyphs
            .entry(font_id.family.clone())
            .or_insert_with(|| available_characters(ui, &font_id, include_missing_emoji));

        let fonts_in_family = fonts_in_family(ui, &font_id.family);

        egui::CollapsingHeader::new(format!(
            "The selected family has {} characters, from {} fonts",
            available_glyphs.len(),
            fonts_in_family.len()
        ))
        .show(ui, |ui| {
            ui.label("In fallback order:");
            for name in fonts_in_family {
                ui.label(egui::RichText::new(name).text_style(egui::TextStyle::Monospace));
            }
        });

        let filter = &self.filter;

        let matching_glyphs: Vec<(char, &GlyphInfo)> = available_glyphs
            .iter()
            .filter(|(chr, glyph_info)| {
                filter.is_empty() || glyph_info.name.contains(filter) || *filter == chr.to_string()
            })
            .map(|(&chr, glyph_info)| (chr, glyph_info))
            .collect();

        ui.separator();

        // Each glyph gets a fixed-size cell so we can calculate what is visible,
        // and only paint those glyphs.
        let spacing = egui::Vec2::splat(2.0);
        let cell_size = egui::Vec2::splat((1.5 * font_id.size).round());

        let num_columns = ((ui.available_width() + spacing.x) / (cell_size.x + spacing.x)).floor();
        let num_columns = (num_columns as usize).max(1);
        let num_rows = matching_glyphs.len().div_ceil(num_columns);

        egui::ScrollArea::vertical().auto_shrink(false).show_rows(
            ui,
            cell_size.y + spacing.y,
            num_rows,
            |ui, rows| {
                ui.spacing_mut().item_spacing = spacing;

                for row in rows {
                    ui.horizontal(|ui| {
                        let start = row * num_columns;
                        let end = (start + num_columns).min(matching_glyphs.len());
                        for &(chr, glyph_info) in &matching_glyphs[start..end] {
                            let button =
                                egui::Button::new(egui::RichText::new(chr).font(font_id.clone()))
                                    .frame(false);

                            let tooltip_ui = |ui: &mut egui::Ui| {
                                char_info_ui(ui, chr, glyph_info, font_id.clone());
                            };

                            if ui
                                .add_sized(cell_size, button)
                                .on_hover_ui(tooltip_ui)
                                .clicked()
                            {
                                ui.copy_text(chr.to_string());
                            }
                        }
                    });
                }
            },
        );
    }
}

fn fonts_in_family(ui: &egui::Ui, family: &egui::FontFamily) -> Vec<String> {
    ui.fonts(|fonts| {
        let mut names = fonts
            .definitions()
            .families
            .get(family)
            .cloned()
            .unwrap_or_default();

        // Fonts found by a `FontProvider`, e.g. among the system fonts:
        names.extend(
            fonts
                .discovered_fonts()
                .iter()
                .filter(|font| font.families.iter().any(|insert| &insert.family == family))
                .map(|font| font.name.clone()),
        );

        names
    })
}

fn char_info_ui(ui: &mut egui::Ui, chr: char, glyph_info: &GlyphInfo, font_id: egui::FontId) {
    let resp = ui.label(egui::RichText::new(chr.to_string()).font(font_id));

    egui::Grid::new("char_info")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Name");
            ui.label(glyph_info.name.clone());
            ui.end_row();

            ui.label("Hex");
            ui.label(format!("{:X}", chr as u32));
            ui.end_row();

            ui.label("Width");
            ui.label(format!("{:.1} pts", resp.rect.width()));
            ui.end_row();

            ui.label("Fonts");
            ui.label(
                format!("{:?}", glyph_info.fonts)
                    .trim_start_matches('[')
                    .trim_end_matches(']'),
            );
            ui.end_row();
        });
}

#[cfg(target_arch = "wasm32")]
struct CanvasTofuDetector {
    canvas: web_sys::HtmlCanvasElement,
    context: web_sys::CanvasRenderingContext2d,
    tofu_cache: std::collections::HashMap<String, Option<Vec<u8>>>,
}

#[cfg(target_arch = "wasm32")]
impl CanvasTofuDetector {
    fn new() -> Option<Self> {
        use wasm_bindgen::JsCast as _;

        let document = web_sys::window()?.document()?;
        let canvas = document
            .create_element("canvas")
            .ok()?
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .ok()?;
        let context = canvas
            .get_context("2d")
            .ok()??
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .ok()?;
        Some(Self {
            canvas,
            context,
            tofu_cache: Default::default(),
        })
    }

    fn draw(&self, text: &str, font: &str) -> Option<Vec<u8>> {
        self.canvas.set_width(64);
        self.canvas.set_height(64);
        self.context.set_font(font);
        self.context.set_text_baseline("alphabetic");
        self.context.set_fill_style_str("white");
        self.context.fill_text(text, 4.0, 48.0).ok()?;
        Some(
            self.context
                .get_image_data(0.0, 0.0, 64.0, 64.0)
                .ok()?
                .data()
                .0,
        )
    }

    fn is_tofu(&mut self, text: &str, family: &egui::FontFamily) -> Option<bool> {
        let family = if family == &egui::FontFamily::Monospace {
            "monospace"
        } else {
            "system-ui, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Noto Color Emoji'"
        };
        let font = format!("32px {family}");
        let candidate = self.draw(text, &font)?;
        if !self.tofu_cache.contains_key(&font) {
            let sample = char::from_u32(0x0378).map(|chr| chr.to_string());
            let sample = sample.as_deref().and_then(|text| self.draw(text, &font));
            self.tofu_cache.insert(font.clone(), sample);
        }
        let tofu = self.tofu_cache.get(&font)?.as_ref()?;
        Some(candidate.as_slice() == tofu.as_slice())
    }
}

fn available_characters(
    ui: &egui::Ui,
    font_id: &egui::FontId,
    include_missing_emoji: bool,
) -> BTreeMap<char, GlyphInfo> {
    #[cfg(target_arch = "wasm32")]
    let mut tofu_detector = include_missing_emoji
        .then(CanvasTofuDetector::new)
        .flatten();
    #[cfg(not(target_arch = "wasm32"))]
    let _ = include_missing_emoji;

    ui.fonts_mut(|f| {
        #[cfg_attr(not(target_arch = "wasm32"), expect(unused_mut))]
        let mut characters: BTreeMap<char, GlyphInfo> = f
            .characters(&font_id.family)
            .iter()
            .filter(|(chr, _fonts)| !chr.is_whitespace() && !chr.is_ascii_control())
            .map(|(chr, fonts)| {
                (
                    *chr,
                    GlyphInfo {
                        name: char_name(*chr),
                        fonts: fonts.clone(),
                    },
                )
            })
            .collect();

        #[cfg(target_arch = "wasm32")]
        if include_missing_emoji {
            for (start, end) in [
                (0x00A9, 0x00A9),
                (0x00AE, 0x00AE),
                (0x203C, 0x2049),
                (0x2122, 0x2139),
                (0x2194, 0x21FF),
                (0x2300, 0x23FF),
                (0x24C2, 0x24C2),
                (0x25AA, 0x25FE),
                (0x2600, 0x27BF),
                (0x2934, 0x2935),
                (0x2B00, 0x2BFF),
                (0x3030, 0x303D),
                (0x3297, 0x3299),
                (0x1F000, 0x1FAFF),
            ] {
                for codepoint in start..=end {
                    let Some(chr) = char::from_u32(codepoint) else {
                        continue;
                    };
                    let text = chr.to_string();
                    let emoji_selector = char::from_u32(0xFE0F).unwrap();
                    let emoji = format!("{chr}{emoji_selector}");
                    if !(egui::has_emoji_presentation(&text)
                        || egui::has_emoji_presentation(&emoji))
                    {
                        continue;
                    }

                    let has_glyph = f.has_glyph(font_id, chr);
                    let include_candidate = if has_glyph {
                        true
                    } else {
                        // Keep candidates when Canvas is unavailable or comparison fails.
                        !tofu_detector
                            .as_mut()
                            .and_then(|detector| detector.is_tofu(&emoji, &font_id.family))
                            .unwrap_or(false)
                    };
                    if include_candidate {
                        characters.entry(chr).or_insert_with(|| GlyphInfo {
                            name: char_name(chr),
                            fonts: vec!["system emoji fallback".to_owned()],
                        });
                    }
                }
            }
        }

        characters
    })
}

fn char_name(chr: char) -> String {
    special_char_name(chr)
        .map(|s| s.to_owned())
        .or_else(|| unicode_names2::name(chr).map(|name| name.to_string().to_lowercase()))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn special_char_name(chr: char) -> Option<&'static str> {
    #[expect(clippy::match_same_arms)] // many "flag"
    match chr {
        // Special private-use-area extensions found in `emoji-icon-font.ttf`:
        // Private use area extensions:
        '\u{FE4E5}' => Some("flag japan"),
        '\u{FE4E6}' => Some("flag usa"),
        '\u{FE4E7}' => Some("flag"),
        '\u{FE4E8}' => Some("flag"),
        '\u{FE4E9}' => Some("flag"),
        '\u{FE4EA}' => Some("flag great britain"),
        '\u{FE4EB}' => Some("flag"),
        '\u{FE4EC}' => Some("flag"),
        '\u{FE4ED}' => Some("flag"),
        '\u{FE4EE}' => Some("flag south korea"),
        '\u{FE82C}' => Some("number sign in square"),
        '\u{FE82E}' => Some("digit one in square"),
        '\u{FE82F}' => Some("digit two in square"),
        '\u{FE830}' => Some("digit three in square"),
        '\u{FE831}' => Some("digit four in square"),
        '\u{FE832}' => Some("digit five in square"),
        '\u{FE833}' => Some("digit six in square"),
        '\u{FE834}' => Some("digit seven in square"),
        '\u{FE835}' => Some("digit eight in square"),
        '\u{FE836}' => Some("digit nine in square"),
        '\u{FE837}' => Some("digit zero in square"),

        // Special private-use-area extensions found in `emoji-icon-font.ttf`:
        // Web services / operating systems / browsers
        '\u{E600}' => Some("web-dribbble"),
        '\u{E601}' => Some("web-stackoverflow"),
        '\u{E602}' => Some("web-vimeo"),
        '\u{E604}' => Some("web-facebook"),
        '\u{E605}' => Some("web-googleplus"),
        '\u{E606}' => Some("web-pinterest"),
        '\u{E607}' => Some("web-tumblr"),
        '\u{E608}' => Some("web-linkedin"),
        '\u{E60A}' => Some("web-stumbleupon"),
        '\u{E60B}' => Some("web-lastfm"),
        '\u{E60C}' => Some("web-rdio"),
        '\u{E60D}' => Some("web-spotify"),
        '\u{E60E}' => Some("web-qq"),
        '\u{E60F}' => Some("web-instagram"),
        '\u{E610}' => Some("web-dropbox"),
        '\u{E611}' => Some("web-evernote"),
        '\u{E612}' => Some("web-flattr"),
        '\u{E613}' => Some("web-skype"),
        '\u{E614}' => Some("web-renren"),
        '\u{E615}' => Some("web-sina-weibo"),
        '\u{E616}' => Some("web-paypal"),
        '\u{E617}' => Some("web-picasa"),
        '\u{E618}' => Some("os-android"),
        '\u{E619}' => Some("web-mixi"),
        '\u{E61A}' => Some("web-behance"),
        '\u{E61B}' => Some("web-circles"),
        '\u{E61C}' => Some("web-vk"),
        '\u{E61D}' => Some("web-smashing"),
        '\u{E61E}' => Some("web-forrst"),
        '\u{E61F}' => Some("os-windows"),
        '\u{E620}' => Some("web-flickr"),
        '\u{E621}' => Some("web-picassa"),
        '\u{E622}' => Some("web-deviantart"),
        '\u{E623}' => Some("web-steam"),
        '\u{E624}' => Some("web-github"),
        '\u{E625}' => Some("web-git"),
        '\u{E626}' => Some("web-blogger"),
        '\u{E627}' => Some("web-soundcloud"),
        '\u{E628}' => Some("web-reddit"),
        '\u{E629}' => Some("web-delicious"),
        '\u{E62A}' => Some("browser-chrome"),
        '\u{E62B}' => Some("browser-firefox"),
        '\u{E62C}' => Some("browser-ie"),
        '\u{E62D}' => Some("browser-opera"),
        '\u{E62E}' => Some("browser-safari"),
        '\u{E62F}' => Some("web-google-drive"),
        '\u{E630}' => Some("web-wordpress"),
        '\u{E631}' => Some("web-joomla"),
        '\u{E632}' => Some("lastfm"),
        '\u{E633}' => Some("web-foursquare"),
        '\u{E634}' => Some("web-yelp"),
        '\u{E635}' => Some("web-drupal"),
        '\u{E636}' => Some("youtube"),
        '\u{F189}' => Some("vk"),
        '\u{F1A6}' => Some("digg"),
        '\u{F1CA}' => Some("web-vine"),
        '\u{F8FF}' => Some("os-apple"),

        // Special private-use-area extensions found in `Ubuntu-Light.ttf`
        '\u{F000}' => Some("uniF000"),
        '\u{F001}' => Some("fi"),
        '\u{F002}' => Some("fl"),
        '\u{F506}' => Some("one seventh"),
        '\u{F507}' => Some("two sevenths"),
        '\u{F508}' => Some("three sevenths"),
        '\u{F509}' => Some("four sevenths"),
        '\u{F50A}' => Some("five sevenths"),
        '\u{F50B}' => Some("six sevenths"),
        '\u{F50C}' => Some("one ninth"),
        '\u{F50D}' => Some("two ninths"),
        '\u{F50E}' => Some("four ninths"),
        '\u{F50F}' => Some("five ninths"),
        '\u{F510}' => Some("seven ninths"),
        '\u{F511}' => Some("eight ninths"),
        '\u{F800}' => Some("zero.alt"),
        '\u{F801}' => Some("one.alt"),
        '\u{F802}' => Some("two.alt"),
        '\u{F803}' => Some("three.alt"),
        '\u{F804}' => Some("four.alt"),
        '\u{F805}' => Some("five.alt"),
        '\u{F806}' => Some("six.alt"),
        '\u{F807}' => Some("seven.alt"),
        '\u{F808}' => Some("eight.alt"),
        '\u{F809}' => Some("nine.alt"),
        '\u{F80A}' => Some("zero.sups"),
        '\u{F80B}' => Some("one.sups"),
        '\u{F80C}' => Some("two.sups"),
        '\u{F80D}' => Some("three.sups"),
        '\u{F80E}' => Some("four.sups"),
        '\u{F80F}' => Some("five.sups"),
        '\u{F810}' => Some("six.sups"),
        '\u{F811}' => Some("seven.sups"),
        '\u{F812}' => Some("eight.sups"),
        '\u{F813}' => Some("nine.sups"),
        '\u{F814}' => Some("zero.sinf"),
        '\u{F815}' => Some("one.sinf"),
        '\u{F816}' => Some("two.sinf"),
        '\u{F817}' => Some("three.sinf"),
        '\u{F818}' => Some("four.sinf"),
        '\u{F819}' => Some("five.sinf"),
        '\u{F81A}' => Some("six.sinf"),
        '\u{F81B}' => Some("seven.sinf"),
        '\u{F81C}' => Some("eight.sinf"),
        '\u{F81D}' => Some("nine.sinf"),

        _ => None,
    }
}
