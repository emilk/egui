use crate::{color::*, label, math::*, widgets::*, Align, Outline, PaintCmd, Region};

/// Showcase some region code
pub struct ExampleApp {
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

impl Default for ExampleApp {
    fn default() -> ExampleApp {
        ExampleApp {
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

impl ExampleApp {
    pub fn ui(&mut self, region: &mut Region) {
        region.foldable("About Emigui", |region| {
            region.add(label!(
                "Emigui is an experimental immediate mode GUI written in Rust."
            ));
        });

        region.foldable("Widgets", |region| {
            region.horizontal(Align::Min, |region| {
                region.add(label!("Text can have").text_color(srgba(110, 255, 110, 255)));
                region.add(label!("color").text_color(srgba(128, 140, 255, 255)));
                region.add(label!("and tooltips (hover me)")).tooltip_text(
                    "This is a multiline tooltip that demonstrates that you can easily add tooltips to any element.\nThis is the second line.\nThis is the third.",
                );
            });

            region.add(Checkbox::new(&mut self.checked, "checkbox"));

            region.horizontal(Align::Min, |region| {
                if region.add(radio(self.radio == 0, "First")).clicked {
                    self.radio = 0;
                }
                if region.add(radio(self.radio == 1, "Second")).clicked {
                    self.radio = 1;
                }
                if region.add(radio(self.radio == 2, "Final")).clicked {
                    self.radio = 2;
                }
            });

            region.horizontal(Align::Min, |region| {
                if region
                    .add(Button::new("Click me"))
                    .tooltip_text("This will just increase a counter.")
                    .clicked
                {
                    self.count += 1;
                }
                region.add(label!(
                    "The button have been clicked {} times",
                    self.count
                ));
            });
        });

        region.foldable("Layouts", |region| {
            region.add(Slider::usize(&mut self.num_columns, 1, 10).text("Columns"));
            region.columns(self.num_columns, |cols| {
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

        region.foldable("Test box rendering", |region| {
            region.add(Slider::f32(&mut self.size.x, 0.0, 500.0).text("width"));
            region.add(Slider::f32(&mut self.size.y, 0.0, 500.0).text("height"));
            region.add(Slider::f32(&mut self.corner_radius, 0.0, 50.0).text("corner_radius"));
            region.add(Slider::f32(&mut self.stroke_width, 0.0, 10.0).text("stroke_width"));
            region.add(Slider::usize(&mut self.num_boxes, 0, 5).text("num_boxes"));

            let pos = region
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
                        pos2(10.0 + pos.x + (i as f32) * (self.size.x * 1.1), pos.y),
                        self.size,
                    ),
                    outline: Some(Outline::new(self.stroke_width, gray(255, 255))),
                });
            }
            region.add_paint_cmds(cmds);
        });

        region.foldable("Slider example", |region| {
            value_ui(&mut self.slider_value, region);
        });

        region.foldable("Name clash example", |region| {
            region.add_label("\
                Regions that store state require unique identifiers so we can track their state between frames. \
                Identifiers are normally derived from the titles of the widget.");

            region.add_label("\
                For instance, foldable regions needs to store wether or not they are open. \
                If you fail to give them unique names then clicking one will open both. \
                To help you debug this, an error message is printed on screen:");

            region.foldable("Foldable", |region| {
                region.add_label("Contents of first folddable region");
            });
            region.foldable("Foldable", |region| {
                region.add_label("Contents of second folddable region");
            });

            region.add_label("\
                Most widgets don't need unique names, but are tracked \
                based on their position on screen. For instance, buttons:");
            region.add(Button::new("Button"));
            region.add(Button::new("Button"));
        });
    }
}

pub fn value_ui(value: &mut usize, region: &mut Region) {
    region.add(Slider::usize(value, 1, 1000));
    if region.add(Button::new("Double it")).clicked {
        *value *= 2;
    }
    region.add(label!("Value: {}", value));
}
