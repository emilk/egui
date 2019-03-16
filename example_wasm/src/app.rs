use emigui::{label, math::*, types::*, widgets::*, Align, Region, TextStyle};

pub fn show_value_gui(value: &mut usize, gui: &mut Region) {
    gui.add(Slider::usize(value, 1, 1000));
    if gui.add(Button::new("Double it")).clicked {
        *value *= 2;
    }
    gui.add(label!("Value: {}", value));
}

pub struct App {
    checked: bool,
    count: usize,
    radio: usize,

    size: Vec2,
    corner_radius: f32,
    stroke_width: f32,
    num_boxes: usize,

    num_columns: usize,

    slider_value: usize,
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
            num_boxes: 1,

            num_columns: 2,

            slider_value: 100,
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

        gui.foldable("Layouts", |gui| {
            gui.add(Slider::usize(&mut self.num_columns, 1, 10).text("Columns"));
            gui.columns(self.num_columns, |cols| {
                for (i, col) in cols.iter_mut().enumerate() {
                    col.add(label!("Column {} out of {}", i + 1, self.num_columns));
                    if i + 1 == self.num_columns {
                        if col.add(Button::new("Delete this")).clicked {
                            self.num_columns -= 1;
                        }
                    }
                }
            });
        });

        gui.foldable("Test box rendering", |gui| {
            gui.add(Slider::f32(&mut self.size.x, 0.0, 500.0).text("width"));
            gui.add(Slider::f32(&mut self.size.y, 0.0, 500.0).text("height"));
            gui.add(Slider::f32(&mut self.corner_radius, 0.0, 50.0).text("corner_radius"));
            gui.add(Slider::f32(&mut self.stroke_width, 0.0, 10.0).text("stroke_width"));
            gui.add(Slider::usize(&mut self.num_boxes, 0, 5).text("num_boxes"));

            let pos = gui
                .reserve_space(
                    vec2(self.size.x * (self.num_boxes as f32), self.size.y),
                    None,
                )
                .rect
                .min();

            let mut cmds = vec![];
            for i in 0..self.num_boxes {
                cmds.push(PaintCmd::Rect {
                    corner_radius: self.corner_radius,
                    fill_color: Some(gray(136, 255)),
                    rect: Rect::from_min_size(
                        vec2(pos.x + (i as f32) * (self.size.x * 1.1), pos.y),
                        self.size,
                    ),
                    outline: Some(Outline {
                        width: self.stroke_width,
                        color: gray(255, 255),
                    }),
                });
            }
            gui.add_paint_cmds(cmds);
        });

        gui.foldable("Slider example", |gui| {
            show_value_gui(&mut self.slider_value, gui);
        });
    }
}
