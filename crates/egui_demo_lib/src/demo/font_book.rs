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
}

impl Default for FontBook {
    fn default() -> Self {
        Self {
            filter: Default::default(),
            font_id: egui::FontId::proportional(18.0),
            available_glyphs: Default::default(),
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
        let available_glyphs = self
            .available_glyphs
            .entry(font_id.family.clone())
            .or_insert_with(|| available_characters(ui, &font_id.family));

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

        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.add(egui::TextEdit::singleline(&mut self.filter).desired_width(120.0));
            self.filter = self.filter.to_lowercase();
            if ui.button("ｘ").clicked() {
                self.filter.clear();
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

fn available_characters(ui: &egui::Ui, family: &egui::FontFamily) -> BTreeMap<char, GlyphInfo> {
    ui.fonts_mut(|f| {
        f.characters(family)
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
            .collect()
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

        _ => None,
    }
}
