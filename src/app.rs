use crate::{layout::Layout, math::*, types::*};

pub trait GuiSettings {
    fn show_gui(&mut self, gui: &mut Layout);
}

pub struct App {
    checked: bool,
    count: i32,
    selected_alternative: i32,

    width: f32,
    height: f32,
    corner_radius: f32,
    stroke_width: f32,
}

impl Default for App {
    fn default() -> App {
        App {
            checked: false,
            selected_alternative: 0,
            count: 0,
            width: 100.0,
            height: 50.0,
            corner_radius: 5.0,
            stroke_width: 2.0,
        }
    }
}

impl GuiSettings for App {
    fn show_gui(&mut self, gui: &mut Layout) {
        gui.checkbox("checkbox", &mut self.checked);

        if gui
            .radio("First alternative", self.selected_alternative == 0)
            .clicked
        {
            self.selected_alternative = 0;
        }
        if gui
            .radio("Second alternative", self.selected_alternative == 1)
            .clicked
        {
            self.selected_alternative = 1;
        }
        if gui
            .radio("Final alternative", self.selected_alternative == 2)
            .clicked
        {
            self.selected_alternative = 2;
        }

        if gui.button("Click me").clicked {
            self.count += 1;
        }

        gui.label(format!("The button have been clicked {} times", self.count));

        gui.foldable("Box rendering options", |gui| {
            gui.slider_f32("width", &mut self.width, 0.0, 500.0);
            gui.slider_f32("height", &mut self.height, 0.0, 500.0);
            gui.slider_f32("corner_radius", &mut self.corner_radius, 0.0, 50.0);
            gui.slider_f32("stroke_width", &mut self.stroke_width, 0.0, 10.0);
        });

        gui.commands
            .push(GuiCmd::PaintCommands(vec![PaintCmd::Rect {
                corner_radius: self.corner_radius,
                fill_color: Some(srgba(136, 136, 136, 255)),
                pos: vec2(300.0, 100.0),
                size: vec2(self.width, self.height),
                outline: Some(Outline {
                    width: self.stroke_width,
                    color: srgba(255, 255, 255, 255),
                }),
            }]));

        gui.foldable("LayoutOptions", |gui| {
            let mut layout_options = gui.layout_options;
            layout_options.show_gui(gui);
            gui.layout_options = layout_options;
        });
    }
}

impl GuiSettings for crate::layout::LayoutOptions {
    fn show_gui(&mut self, gui: &mut Layout) {
        if gui.button("Reset LayoutOptions").clicked {
            *self = Default::default();
        }
        gui.slider_f32("item_spacing.x", &mut self.item_spacing.x, 0.0, 10.0);
        gui.slider_f32("item_spacing.y", &mut self.item_spacing.y, 0.0, 10.0);
        gui.slider_f32("width", &mut self.width, 0.0, 1000.0);
        gui.slider_f32("button_height", &mut self.button_height, 0.0, 60.0);
        gui.slider_f32(
            "checkbox_radio_height",
            &mut self.checkbox_radio_height,
            0.0,
            60.0,
        );
        gui.slider_f32("slider_height", &mut self.slider_height, 0.0, 60.0);
    }
}

impl GuiSettings for crate::style::Style {
    fn show_gui(&mut self, gui: &mut Layout) {
        if gui.button("Reset Style").clicked {
            *self = Default::default();
        }
        gui.checkbox("debug_rects", &mut self.debug_rects);
        gui.slider_f32("line_width", &mut self.line_width, 0.0, 10.0);
        gui.slider_f32("font_size", &mut self.font_size, 5.0, 32.0);
    }
}
