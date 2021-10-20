use egui::TextStyle;

use crate::syntax_highlighting::code_view_ui;

#[derive(Debug)]
pub struct CodeSamples {
    name: String,
    age: u32,
}

impl Default for CodeSamples {
    fn default() -> Self {
        Self {
            name: "Arthur".to_owned(),
            age: 42,
        }
    }
}

impl CodeSamples {
    fn samples_in_grid(&mut self, ui: &mut egui::Ui) {
        use crate::before_end_row;

        code_view_ui(ui, &before_end_row!());
        ui.heading("Code samples");
        ui.end_row();

        code_view_ui(ui, &before_end_row!());
        // Putting things on the same line using ui.horizontal:
        ui.horizontal(|ui| {
            ui.label("Your name: ");
            ui.text_edit_singleline(&mut self.name);
        });
        ui.end_row();

        code_view_ui(ui, &before_end_row!());
        ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
        ui.end_row();

        code_view_ui(ui, &before_end_row!());
        if ui.button("Click each year").clicked() {
            self.age += 1;
        }
        ui.end_row();

        code_view_ui(ui, &before_end_row!());
        ui.label(format!("Hello '{}', age {}", self.name, self.age));
        ui.end_row();
    }
}

impl super::Demo for CodeSamples {
    fn name(&self) -> &'static str {
        "🖮 Code Samples"
    }

    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool) {
        use super::View;
        egui::Window::new(self.name())
            .open(open)
            .default_size([800.0, 400.0])
            .vscroll(false)
            .hscroll(true)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for CodeSamples {
    fn ui(&mut self, ui: &mut egui::Ui) {
        code_view_ui(ui, &format!("{:#?}", self));
        ui.separator();

        code_view_ui(
            ui,
            "impl CodeSamples {\n    fn ui(&mut self, ui: &mut egui::Ui) {",
        );

        // TODO: wrap in scrollarea
        // egui::ScrollArea::vertical().show(ui, |ui| {

        ui.horizontal(|ui| {
            let indentation = 8.0 * ui.fonts()[TextStyle::Monospace].glyph_width(' ');
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

        code_view_ui(ui, "    }\n}");

        ui.separator();
        let mut theme = crate::syntax_highlighting::CodeTheme::from_memory(ui.ctx());
        ui.collapsing("Theme", |ui| {
            theme.ui(ui);
            theme.store_in_memory(ui.ctx());
        });
    }
}

#[macro_export]
macro_rules! file_contents {
    () => {{
        include_str!(concat!("../../../../", file!()))
    }};
}

#[macro_export]
macro_rules! next_n_lines {
    ($num: expr) => {{
        let full_source = $crate::file_contents!();

        let first_line = line!() + 1;
        let last_line = first_line + $num;

        let mut line_nr = 1;

        let mut start_idx = 0;
        let mut end_idx = 0;

        for (i, c) in full_source.bytes().enumerate() {
            if c == b'\n' {
                line_nr += 1;
                if line_nr == first_line {
                    start_idx = i + 1;
                }
                if line_nr == last_line {
                    end_idx = i;
                    break;
                }
            }
        }

        remove_leading_indentation(&full_source[start_idx..end_idx])
    }};
}

#[macro_export]
macro_rules! code_before {
    ($end_pattern: literal) => {{
        let full_source = $crate::file_contents!();

        let first_line = line!() + 1;

        let mut line_nr = 1;

        let mut start_idx = 0;

        for (i, c) in full_source.bytes().enumerate() {
            if c == b'\n' {
                line_nr += 1;
                if line_nr == first_line {
                    start_idx = i + 1;
                    break;
                }
            }
        }
        let snippet = &full_source[start_idx..];
        let end = snippet.find($end_pattern).unwrap_or_else(|| snippet.len());
        let snippet = snippet[..end].trim_end();

        remove_leading_indentation(snippet)
    }};
}

#[macro_export]
macro_rules! before_end_row {
    () => {{
        $crate::code_before!("ui.end_row()")
    }};
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
            .map(|endline| endline + 1)
            .unwrap_or_else(|| code.len());
        out += &code[start..end];
        code = &code[end..];
    }
    out
}
