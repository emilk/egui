use std::collections::BTreeMap;

pub struct FontBook {
    filter: String,
    text_style: egui::TextStyle,
    named_chars: BTreeMap<egui::TextStyle, BTreeMap<char, String>>,
}

impl Default for FontBook {
    fn default() -> Self {
        Self {
            filter: Default::default(),
            text_style: egui::TextStyle::Button,
            named_chars: Default::default(),
        }
    }
}

impl super::Demo for FontBook {
    fn name(&self) -> &'static str {
        "🔤 Font Book"
    }

    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool) {
        egui::Window::new(self.name()).open(open).show(ctx, |ui| {
            use super::View;
            self.ui(ui);
        });
    }
}

impl super::View for FontBook {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label(format!(
            "The selected font supports {} characters.",
            self.named_chars
                .get(&self.text_style)
                .map(|map| map.len())
                .unwrap_or_default()
        ));

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("You can add more characters by installing additional fonts with ");
            ui.add(
                egui::Hyperlink::from_label_and_url(
                    "Context::set_fonts",
                    "https://docs.rs/egui/latest/egui/struct.Context.html#method.set_fonts",
                )
                .text_style(egui::TextStyle::Monospace),
            );
            ui.label(".");
        });

        ui.separator();

        egui::ComboBox::from_label("Text style")
            .selected_text(format!("{:?}", self.text_style))
            .show_ui(ui, |ui| {
                for style in egui::TextStyle::all() {
                    ui.selectable_value(&mut self.text_style, style, format!("{:?}", style));
                }
            });

        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter);
            self.filter = self.filter.to_lowercase();
            if ui.button("ｘ").clicked() {
                self.filter.clear();
            }
        });

        let text_style = self.text_style;
        let filter = &self.filter;
        let named_chars = self.named_chars.entry(text_style).or_insert_with(|| {
            ui.fonts()[text_style]
                .characters()
                .iter()
                .filter(|chr| !chr.is_whitespace() && !chr.is_ascii_control())
                .map(|&chr| (chr, char_name(chr)))
                .collect()
        });

        ui.separator();

        egui::ScrollArea::auto_sized().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::Vec2::splat(2.0);

                for (&chr, name) in named_chars {
                    if filter.is_empty() || name.contains(filter) || *filter == chr.to_string() {
                        let button = egui::Button::new(chr).text_style(text_style).frame(false);

                        let tooltip_ui = |ui: &mut egui::Ui| {
                            ui.add(egui::Label::new(chr).text_style(text_style));
                            ui.label(format!("{}\nU+{:X}\n\nClick to copy", name, chr as u32));
                        };

                        if ui.add(button).on_hover_ui(tooltip_ui).clicked() {
                            ui.output().copied_text = chr.to_string();
                        }
                    }
                }
            });
        });
    }
}

fn char_name(chr: char) -> String {
    unicode_names2::name(chr)
        .map(|name| name.to_string().to_lowercase())
        .unwrap_or_else(|| {
            #[allow(clippy::match_same_arms)]
            match chr {
                // Special private-use-area extensions found in `emoji-icon-font.ttf`:
                // Private use area extensions:
                '\u{FE4E5}' => "flag japan".to_owned(),
                '\u{FE4E6}' => "flag usa".to_owned(),
                '\u{FE4E7}' => "flag".to_owned(),
                '\u{FE4E8}' => "flag".to_owned(),
                '\u{FE4E9}' => "flag".to_owned(),
                '\u{FE4EA}' => "flag great britain".to_owned(),
                '\u{FE4EB}' => "flag".to_owned(),
                '\u{FE4EC}' => "flag".to_owned(),
                '\u{FE4ED}' => "flag".to_owned(),
                '\u{FE4EE}' => "flag south korea".to_owned(),
                '\u{FE82C}' => "number sign in square".to_owned(),
                '\u{FE82E}' => "digit one in square".to_owned(),
                '\u{FE82F}' => "digit two in square".to_owned(),
                '\u{FE830}' => "digit three in square".to_owned(),
                '\u{FE831}' => "digit four in square".to_owned(),
                '\u{FE832}' => "digit five in square".to_owned(),
                '\u{FE833}' => "digit six in square".to_owned(),
                '\u{FE834}' => "digit seven in square".to_owned(),
                '\u{FE835}' => "digit eight in square".to_owned(),
                '\u{FE836}' => "digit nine in square".to_owned(),
                '\u{FE837}' => "digit zero in square".to_owned(),

                // Special private-use-area extensions found in `emoji-icon-font.ttf`:
                // Web services / operating systems / browsers
                '\u{E600}' => "web-dribbble".to_owned(),
                '\u{E601}' => "web-stackoverflow".to_owned(),
                '\u{E602}' => "web-vimeo".to_owned(),
                '\u{E603}' => "web-twitter".to_owned(),
                '\u{E604}' => "web-facebook".to_owned(),
                '\u{E605}' => "web-googleplus".to_owned(),
                '\u{E606}' => "web-pinterest".to_owned(),
                '\u{E607}' => "web-tumblr".to_owned(),
                '\u{E608}' => "web-linkedin".to_owned(),
                '\u{E60A}' => "web-stumbleupon".to_owned(),
                '\u{E60B}' => "web-lastfm".to_owned(),
                '\u{E60C}' => "web-rdio".to_owned(),
                '\u{E60D}' => "web-spotify".to_owned(),
                '\u{E60E}' => "web-qq".to_owned(),
                '\u{E60F}' => "web-instagram".to_owned(),
                '\u{E610}' => "web-dropbox".to_owned(),
                '\u{E611}' => "web-evernote".to_owned(),
                '\u{E612}' => "web-flattr".to_owned(),
                '\u{E613}' => "web-skype".to_owned(),
                '\u{E614}' => "web-renren".to_owned(),
                '\u{E615}' => "web-sina-weibo".to_owned(),
                '\u{E616}' => "web-paypal".to_owned(),
                '\u{E617}' => "web-picasa".to_owned(),
                '\u{E618}' => "os-android".to_owned(),
                '\u{E619}' => "web-mixi".to_owned(),
                '\u{E61A}' => "web-behance".to_owned(),
                '\u{E61B}' => "web-circles".to_owned(),
                '\u{E61C}' => "web-vk".to_owned(),
                '\u{E61D}' => "web-smashing".to_owned(),
                '\u{E61E}' => "web-forrst".to_owned(),
                '\u{E61F}' => "os-windows".to_owned(),
                '\u{E620}' => "web-flickr".to_owned(),
                '\u{E621}' => "web-picassa".to_owned(),
                '\u{E622}' => "web-deviantart".to_owned(),
                '\u{E623}' => "web-steam".to_owned(),
                '\u{E624}' => "web-github".to_owned(),
                '\u{E625}' => "web-git".to_owned(),
                '\u{E626}' => "web-blogger".to_owned(),
                '\u{E627}' => "web-soundcloud".to_owned(),
                '\u{E628}' => "web-reddit".to_owned(),
                '\u{E629}' => "web-delicious".to_owned(),
                '\u{E62A}' => "browser-chrome".to_owned(),
                '\u{E62B}' => "browser-firefox".to_owned(),
                '\u{E62C}' => "browser-ie".to_owned(),
                '\u{E62D}' => "browser-opera".to_owned(),
                '\u{E62E}' => "browser-safari".to_owned(),
                '\u{E62F}' => "web-google-drive".to_owned(),
                '\u{E630}' => "web-wordpress".to_owned(),
                '\u{E631}' => "web-joomla".to_owned(),
                '\u{E632}' => "lastfm".to_owned(),
                '\u{E633}' => "web-foursquare".to_owned(),
                '\u{E634}' => "web-yelp".to_owned(),
                '\u{E635}' => "web-drupal".to_owned(),
                '\u{E636}' => "youtube".to_owned(),
                '\u{F189}' => "vk".to_owned(),
                '\u{F1A6}' => "digg".to_owned(),
                '\u{F1CA}' => "web-vine".to_owned(),
                '\u{F8FF}' => "os-apple".to_owned(),

                // Special private-use-area extensions found in `Ubuntu-Light.ttf`
                '\u{F000}' => "uniF000".to_owned(),
                '\u{F001}' => "fi".to_owned(),
                '\u{F002}' => "fl".to_owned(),
                '\u{F506}' => "one seventh".to_owned(),
                '\u{F507}' => "two sevenths".to_owned(),
                '\u{F508}' => "three sevenths".to_owned(),
                '\u{F509}' => "four sevenths".to_owned(),
                '\u{F50A}' => "fiv esevenths".to_owned(),
                '\u{F50B}' => "six sevenths".to_owned(),
                '\u{F50C}' => "one ninth".to_owned(),
                '\u{F50D}' => "two ninths".to_owned(),
                '\u{F50E}' => "four ninths".to_owned(),
                '\u{F50F}' => "five ninths".to_owned(),
                '\u{F510}' => "seven ninths".to_owned(),
                '\u{F511}' => "eight ninths".to_owned(),
                '\u{F800}' => "zero.alt".to_owned(),
                '\u{F801}' => "one.alt".to_owned(),
                '\u{F802}' => "two.alt".to_owned(),
                '\u{F803}' => "three.alt".to_owned(),
                '\u{F804}' => "four.alt".to_owned(),
                '\u{F805}' => "five.alt".to_owned(),
                '\u{F806}' => "six.alt".to_owned(),
                '\u{F807}' => "seven.alt".to_owned(),
                '\u{F808}' => "eight.alt".to_owned(),
                '\u{F809}' => "nine.alt".to_owned(),
                '\u{F80A}' => "zero.sups".to_owned(),
                '\u{F80B}' => "one.sups".to_owned(),
                '\u{F80C}' => "two.sups".to_owned(),
                '\u{F80D}' => "three.sups".to_owned(),
                '\u{F80E}' => "four.sups".to_owned(),
                '\u{F80F}' => "five.sups".to_owned(),
                '\u{F810}' => "six.sups".to_owned(),
                '\u{F811}' => "seven.sups".to_owned(),
                '\u{F812}' => "eight.sups".to_owned(),
                '\u{F813}' => "nine.sups".to_owned(),
                '\u{F814}' => "zero.sinf".to_owned(),
                '\u{F815}' => "one.sinf".to_owned(),
                '\u{F816}' => "two.sinf".to_owned(),
                '\u{F817}' => "three.sinf".to_owned(),
                '\u{F818}' => "four.sinf".to_owned(),
                '\u{F819}' => "five.sinf".to_owned(),
                '\u{F81A}' => "six.sinf".to_owned(),
                '\u{F81B}' => "seven.sinf".to_owned(),
                '\u{F81C}' => "eight.sinf".to_owned(),
                '\u{F81D}' => "nine.sinf".to_owned(),

                _ => "unknown".to_owned(),
            }
        })
}
