#[derive(Clone, Copy, Debug, PartialEq)]
enum WidgetType {
    Label,
    Button,
    TextEdit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ManualLayoutTest {
    widget_offset: egui::Vec2,
    widget_size: egui::Vec2,
    widget_type: WidgetType,
    text_edit_contents: String,
}

impl Default for ManualLayoutTest {
    fn default() -> Self {
        Self {
            widget_offset: egui::Vec2::splat(150.0),
            widget_size: egui::vec2(200.0, 100.0),
            widget_type: WidgetType::Button,
            text_edit_contents: crate::LOREM_IPSUM.to_owned(),
        }
    }
}

impl crate::Demo for ManualLayoutTest {
    fn name(&self) -> &'static str {
        "Manual Layout Test"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .resizable(false)
            .open(open)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for ManualLayoutTest {
    fn ui(&mut self, ui: &mut egui::Ui) {
        egui::reset_button(ui, self, "Reset");

        let Self {
            widget_offset,
            widget_size,
            widget_type,
            text_edit_contents,
        } = self;
        ui.horizontal(|ui| {
            ui.label("Test widget:");
            ui.radio_value(widget_type, WidgetType::Button, "Button");
            ui.radio_value(widget_type, WidgetType::Label, "Label");
            ui.radio_value(widget_type, WidgetType::TextEdit, "TextEdit");
        });
        egui::Grid::new("pos_size").show(ui, |ui| {
            ui.label("Widget position:");
            ui.add(egui::Slider::new(&mut widget_offset.x, 0.0..=400.0));
            ui.add(egui::Slider::new(&mut widget_offset.y, 0.0..=400.0));
            ui.end_row();

            ui.label("Widget size:");
            ui.add(egui::Slider::new(&mut widget_size.x, 0.0..=400.0));
            ui.add(egui::Slider::new(&mut widget_size.y, 0.0..=400.0));
            ui.end_row();
        });

        let widget_rect =
            egui::Rect::from_min_size(ui.min_rect().min + *widget_offset, *widget_size);

        ui.add(crate::egui_github_link_file!());

        // Showing how to place a widget anywhere in the [`Ui`]:
        match *widget_type {
            WidgetType::Button => {
                ui.put(widget_rect, egui::Button::new("Example button"));
            }
            WidgetType::Label => {
                ui.put(widget_rect, egui::Label::new("Example label"));
            }
            WidgetType::TextEdit => {
                ui.put(widget_rect, egui::TextEdit::multiline(text_edit_contents));
            }
        }
    }
}
