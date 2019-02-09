use emigui::{label, math::*, types::*, widgets::*, Align, Region, TextStyle};

pub struct App {
    checked: bool,
    count: i32,
    radio: i32,

    size: Vec2,
    corner_radius: f32,
    stroke_width: f32,
}

impl Default for App {
    fn default() -> App {
        App {
            checked: true,
            radio: 0,
            count: 0,
            size: vec2(100.0, 50.0),
            corner_radius: 5.0,
            stroke_width: 2.0,
        }
    }
}

impl App {
    pub fn show_gui(&mut self, gui: &mut Region) {
        gui.add(label!("Emigui!").text_style(TextStyle::Heading));
        gui.add(label!("Emigui is an Immediate mode GUI written in Rust, compiled to WebAssembly, rendered with WebGL."));
        gui.add(Separator::new());

        gui.foldable("Widget examples", |gui| {
            gui.horizontal(Align::Min, |gui| {
                gui.add(label!("Text can have").text_color(srgba(110, 255, 110, 255)));
                gui.add(label!("color").text_color(srgba(128, 140, 255, 255)));
                gui.add(label!("and tooltips (hover me)")).tooltip_text(
                    "This is a multiline tooltip that demonstrates that you can easily add tooltips to any element.\nThis is the second line.\nThis is the third.",
                );
            });

            gui.add(Checkbox::new(&mut self.checked, "checkbox"));

            gui.horizontal(Align::Min, |gui| {
                if gui.add(radio(self.radio == 0, "First")).clicked {
                    self.radio = 0;
                }
                if gui.add(radio(self.radio == 1, "Second")).clicked {
                    self.radio = 1;
                }
                if gui.add(radio(self.radio == 2, "Final")).clicked {
                    self.radio = 2;
                }
            });

            gui.horizontal(Align::Min, |gui| {
                if gui
                    .add(Button::new("Click me"))
                    .tooltip_text("This will just increase a counter.")
                    .clicked
                {
                    self.count += 1;
                }
                gui.add(label!(
                    "The button have been clicked {} times",
                    self.count
                ));
            });
        });

        gui.foldable("Test box rendering", |gui| {
            gui.add(Slider::new(&mut self.size.x, 0.0, 500.0).text("width"));
            gui.add(Slider::new(&mut self.size.y, 0.0, 500.0).text("height"));
            gui.add(Slider::new(&mut self.corner_radius, 0.0, 50.0).text("corner_radius"));
            gui.add(Slider::new(&mut self.stroke_width, 0.0, 10.0).text("stroke_width"));

            let pos = gui.reserve_space(self.size, None).rect.min();
            gui.add_graphic(GuiCmd::PaintCommands(vec![PaintCmd::Rect {
                corner_radius: self.corner_radius,
                fill_color: Some(srgba(136, 136, 136, 255)),
                rect: Rect::from_min_size(pos, self.size),
                outline: Some(Outline {
                    width: self.stroke_width,
                    color: srgba(255, 255, 255, 255),
                }),
            }]));
        });
    }
}
