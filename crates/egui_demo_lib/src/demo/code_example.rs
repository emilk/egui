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
        // Note: we keep the code narrow so that the example fits on a mobile screen.

        let Self { name, age } = self; // for brevity later on

        show_code(ui, r#"ui.heading("Example");"#);
        ui.heading("Example");
        ui.end_row();

        show_code(
            ui,
            r#"
            ui.horizontal(|ui| {
                ui.label("Name");
                ui.text_edit_singleline(name);
            });"#,
        );
        // Putting things on the same line using ui.horizontal:
        ui.horizontal(|ui| {
            ui.label("Name");
            ui.text_edit_singleline(name);
        });
        ui.end_row();

        show_code(
            ui,
            r#"
            ui.add(
                egui::DragValue::new(age)
                    .range(0..=120)
                    .suffix(" years"),
            );"#,
        );
        ui.add(egui::DragValue::new(age).range(0..=120).suffix(" years"));
        ui.end_row();

        show_code(
            ui,
            r#"
            if ui.button("Increment").clicked() {
                *age += 1;
            }"#,
        );
        if ui.button("Increment").clicked() {
            *age += 1;
        }
        ui.end_row();

        #[expect(clippy::literal_string_with_formatting_args)]
        show_code(ui, r#"ui.label(format!("{name} is {age}"));"#);
        ui.label(format!("{name} is {age}"));
        ui.end_row();
    }

    fn code(&mut self, ui: &mut egui::Ui) {
        show_code(
            ui,
            r"
pub struct CodeExample {
    name: String,
    age: u32,
}

impl CodeExample {
    fn ui(&mut self, ui: &mut egui::Ui) {
        // Saves us from writing `&mut self.name` etc
        let Self { name, age } = self;",
        );

        ui.horizontal(|ui| {
            let font_id = egui::TextStyle::Monospace.resolve(ui.style());
            let indentation = 2.0 * 4.0 * ui.fonts_mut(|f| f.glyph_width(&font_id, ' '));
            ui.add_space(indentation);

            egui::Grid::new("code_samples")
                .striped(true)
                .num_columns(2)
                .show(ui, |ui| {
                    self.samples_in_grid(ui);
                });
        });

        crate::rust_view_ui(ui, "    }\n}");
    }
}

impl crate::Demo for CodeExample {
    fn name(&self) -> &'static str {
        "🖮 Code Example"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        use crate::View as _;
        egui::Window::new(self.name())
            .open(open)
            .min_width(375.0)
            .default_size([390.0, 500.0])
            .scroll(false)
            .resizable([true, false]) // resizable so we can shrink if the text edit grows
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| self.ui(ui));
    }
}

impl crate::View for CodeExample {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
            self.code(ui);
        });

        ui.separator();

        crate::rust_view_ui(ui, &format!("{self:#?}"));

        ui.separator();

        let mut theme =
            egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());
        ui.collapsing("Theme", |ui| {
            theme.ui(ui);
            theme.store_in_memory(ui.ctx());
        });

        ui.separator();

        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
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
