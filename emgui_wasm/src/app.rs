use emgui::{math::*, types::*, Region};

pub trait GuiSettings {
    fn show_gui(&mut self, gui: &mut Region);
}

pub struct App {
    checked: bool,
    count: i32,
    selected_alternative: i32,

    size: Vec2,
    corner_radius: f32,
    stroke_width: f32,
}

impl Default for App {
    fn default() -> App {
        App {
            checked: true,
            selected_alternative: 0,
            count: 0,
            size: vec2(100.0, 50.0),
            corner_radius: 5.0,
            stroke_width: 2.0,
        }
    }
}

impl GuiSettings for App {
    fn show_gui(&mut self, gui: &mut Region) {
        gui.label(format!(
            "Screen size: {} x {}",
            gui.input().screen_size.x,
            gui.input().screen_size.y,
        ));

        gui.label("Hover me").tooltip_text(
            "This is a multiline tooltip that demonstrates that you can easily add tooltips to any element.\nThis is the second line.\nThis is the third.",
        );

        gui.checkbox("checkbox", &mut self.checked);

        gui.horizontal(|gui| {
            if gui.radio("First", self.selected_alternative == 0).clicked {
                self.selected_alternative = 0;
            }
            if gui.radio("Second", self.selected_alternative == 1).clicked {
                self.selected_alternative = 1;
            }
            if gui.radio("Final", self.selected_alternative == 2).clicked {
                self.selected_alternative = 2;
            }
        });

        if gui
            .button("Click me")
            .tooltip_text("This will just increase a counter.")
            .clicked
        {
            self.count += 1;
        }

        gui.label(format!("This is a multiline label.\nThe button have been clicked {} times.\nBelow are more options.", self.count));

        gui.foldable("Test box rendering", |gui| {
            gui.slider_f32("width", &mut self.size.x, 0.0, 500.0);
            gui.slider_f32("height", &mut self.size.y, 0.0, 500.0);
            gui.slider_f32("corner_radius", &mut self.corner_radius, 0.0, 50.0);
            gui.slider_f32("stroke_width", &mut self.stroke_width, 0.0, 10.0);

            let pos = gui.cursor();
            gui.add_graphic(GuiCmd::PaintCommands(vec![PaintCmd::Rect {
                corner_radius: self.corner_radius,
                fill_color: Some(srgba(136, 136, 136, 255)),
                pos,
                size: self.size,
                outline: Some(Outline {
                    width: self.stroke_width,
                    color: srgba(255, 255, 255, 255),
                }),
            }]));
            gui.reserve_space(self.size, None);
        });
    }
}

impl GuiSettings for emgui::LayoutOptions {
    fn show_gui(&mut self, gui: &mut Region) {
        if gui.button("Reset LayoutOptions").clicked {
            *self = Default::default();
        }
        gui.slider_f32("item_spacing.x", &mut self.item_spacing.x, 0.0, 10.0);
        gui.slider_f32("item_spacing.y", &mut self.item_spacing.y, 0.0, 10.0);
        gui.slider_f32("window_padding.x", &mut self.window_padding.x, 0.0, 10.0);
        gui.slider_f32("window_padding.y", &mut self.window_padding.y, 0.0, 10.0);
        gui.slider_f32("indent", &mut self.indent, 0.0, 100.0);
        gui.slider_f32("button_padding.x", &mut self.button_padding.x, 0.0, 20.0);
        gui.slider_f32("button_padding.y", &mut self.button_padding.y, 0.0, 20.0);
        gui.slider_f32("start_icon_width", &mut self.start_icon_width, 0.0, 60.0);
    }
}

impl GuiSettings for emgui::Style {
    fn show_gui(&mut self, gui: &mut Region) {
        if gui.button("Reset Style").clicked {
            *self = Default::default();
        }
        gui.checkbox("debug_rects", &mut self.debug_rects);
        gui.slider_f32("line_width", &mut self.line_width, 0.0, 10.0);
        gui.slider_f32("font_size", &mut self.font_size, 5.0, 32.0);
    }
}
