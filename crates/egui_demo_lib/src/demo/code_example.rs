#[derive(Debug)]
pub struct CodeExample {
    name: String,
    age: u32,
}

impl Default for CodeExample {
    fn default() -> Self {
        Self {
            name: "Arthur".to_owned(),
            age: 42,
        }
    }
}

impl CodeExample {
    fn samples_in_grid(&mut self, ui: &mut egui::Ui) {
        show_code(ui, r#"ui.heading("Code samples");"#);
        ui.heading("Code samples");
        ui.end_row();

        show_code(
            ui,
            r#"
            // Putting things on the same line using ui.horizontal:
            ui.horizontal(|ui| {
                ui.label("Your name: ");
                ui.text_edit_singleline(&mut self.name);
            });"#,
        );
        // Putting things on the same line using ui.horizontal:
        ui.horizontal(|ui| {
            ui.label("Your name: ");
            ui.text_edit_singleline(&mut self.name);
        });
        ui.end_row();

        show_code(
            ui,
            r#"ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));"#,
        );
        ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
        ui.end_row();

        show_code(
            ui,
            r#"
            if ui.button("Increment").clicked() {
                self.age += 1;
            }"#,
        );
        if ui.button("Increment").clicked() {
            self.age += 1;
        }
        ui.end_row();

        show_code(
            ui,
            r#"ui.label(format!("Hello '{}', age {}", self.name, self.age));"#,
        );
        ui.label(format!("Hello '{}', age {}", self.name, self.age));
        ui.end_row();
    }
}

impl super::Demo for CodeExample {
    fn name(&self) -> &'static str {
        "🖮 Code Example"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        use super::View;
        egui::Window::new(self.name())
            .open(open)
            .default_size([800.0, 400.0])
            .vscroll(false)
            .hscroll(true)
            .resizable([true, false])
            .show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for CodeExample {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });

        crate::rust_view_ui(
            ui,
            r"
pub struct CodeExample {
    name: String,
    age: u32,
}

impl CodeExample {
    fn ui(&mut self, ui: &mut egui::Ui) {
"
            .trim(),
        );

        ui.horizontal(|ui| {
            let font_id = egui::TextStyle::Monospace.resolve(ui.style());
            let indentation = 8.0 * ui.fonts(|f| f.glyph_width(&font_id, ' '));
            let item_spacing = ui.spacing_mut().item_spacing;
            ui.add_space(indentation - item_spacing.x);

            egui::Grid::new("code_samples")
                .striped(true)
                .num_columns(2)
                .min_col_width(16.0)
                .spacing([16.0, 8.0])
                .show(ui, |ui| {
                    self.samples_in_grid(ui);
                });
        });

        crate::rust_view_ui(ui, "    }\n}");

        ui.separator();

        crate::rust_view_ui(ui, &format!("{self:#?}"));

        ui.separator();

        let mut theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx());
        ui.collapsing("Theme", |ui| {
            theme.ui(ui);
            theme.store_in_memory(ui.ctx());
        });
    }
}

fn show_code(ui: &mut egui::Ui, code: &str) {
    let code = remove_leading_indentation(code.trim_start_matches('\n'));
    crate::rust_view_ui(ui, &code);
}

fn remove_leading_indentation(code: &str) -> String {
    fn is_indent(c: &u8) -> bool {
        matches!(*c, b' ' | b'\t')
    }

    let first_line_indent = code.bytes().take_while(is_indent).count();

    let mut out = String::new();

    let mut code = code;
    while !code.is_empty() {
        let indent = code.bytes().take_while(is_indent).count();
        let start = first_line_indent.min(indent);
        let end = code
            .find('\n')
            .map_or_else(|| code.len(), |endline| endline + 1);
        out += &code[start..end];
        code = &code[end..];
    }
    out
}
