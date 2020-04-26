use crate::{color::*, containers::*, widgets::*, *};

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

    painting: Painting,
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

            painting: Default::default(),
        }
    }
}

impl ExampleApp {
    pub fn ui(&mut self, region: &mut Region) {
        region.collapsing("About Emigui", |region| {
            region.add(label!(
                "Emigui is an experimental immediate mode GUI written in Rust."
            ));

            region.horizontal(Align::Min, |region| {
                region.add_label("Project home page:");
                region.add_hyperlink("https://github.com/emilk/emigui/");
            });
        });

        CollapsingHeader::new("Widgets")
            // .default_open()
            .show(region, |region| {
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

            region.add(Slider::usize(&mut self.slider_value, 1..=1000).text("value"));
            if region.add(Button::new("Double it")).clicked {
                self.slider_value *= 2;
            }
        });

        region.collapsing("Layouts", |region| {
            region.add(Slider::usize(&mut self.num_columns, 1..=10).text("Columns"));
            region.columns(self.num_columns, |cols| {
                for (i, col) in cols.iter_mut().enumerate() {
                    col.add(label!("Column {} out of {}", i + 1, self.num_columns));
                    if i + 1 == self.num_columns && col.add(Button::new("Delete this")).clicked {
                        self.num_columns -= 1;
                    }
                }
            });
        });

        region.collapsing("Test box rendering", |region| {
            region.add(Slider::f32(&mut self.size.x, 0.0..=500.0).text("width"));
            region.add(Slider::f32(&mut self.size.y, 0.0..=500.0).text("height"));
            region.add(Slider::f32(&mut self.corner_radius, 0.0..=50.0).text("corner_radius"));
            region.add(Slider::f32(&mut self.stroke_width, 0.0..=10.0).text("stroke_width"));
            region.add(Slider::usize(&mut self.num_boxes, 0..=5).text("num_boxes"));

            let pos = region
                .reserve_space(
                    vec2(self.size.x * (self.num_boxes as f32), self.size.y),
                    None,
                )
                .rect
                .min;

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

        CollapsingHeader::new("Scroll area")
            // .default_open()
            .show(region, |region| {
                ScrollArea::default().show(region, |region| {
                    region.add_label(LOREM_IPSUM);
                });
            });

        CollapsingHeader::new("Painting")
            // .default_open()
            .show(region, |region| self.painting.ui(region));

        CollapsingHeader::new("Resize")
            .default_open()
            .show(region, |region| {
                Resize::default()
                    .default_height(200.0)
                    // .as_wide_as_possible()
                    .auto_shrink_height(false)
                    .show(region, |region| {
                        region.add(label!("This region can be resized!"));
                        region.add(label!("Just pull the handle on the bottom right"));
                    });
            });

        region.collapsing("Name clash example", |region| {
            region.add_label("\
                Regions that store state require unique identifiers so we can track their state between frames. \
                Identifiers are normally derived from the titles of the widget.");

            region.add_label("\
                For instance, collapsing regions needs to store wether or not they are open. \
                If you fail to give them unique names then clicking one will open both. \
                To help you debug this, an error message is printed on screen:");

            region.collapsing("Collapsing header", |region| {
                region.add_label("Contents of first folddable region");
            });
            region.collapsing("Collapsing header", |region| {
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

#[derive(Default)]
struct Painting {
    lines: Vec<Vec<Vec2>>,
}

impl Painting {
    pub fn ui(&mut self, region: &mut Region) {
        region.add_label("Draw with your mouse to paint");
        if region.add(Button::new("Clear")).clicked {
            self.lines.clear();
        }

        region.add_custom_contents(vec2(f32::INFINITY, 200.0), |region| {
            let canvas_corner = region.cursor();
            let interact = region.reserve_space(region.available_space(), Some(region.id));
            region.clip_rect = region.clip_rect.intersect(&interact.rect); // Make sure we don't paint out of bounds

            if self.lines.is_empty() {
                self.lines.push(vec![]);
            }

            let current_line = self.lines.last_mut().unwrap();

            if interact.active {
                if let Some(mouse_pos) = region.input().mouse_pos {
                    let canvas_pos = mouse_pos - canvas_corner;
                    if current_line.last() != Some(&canvas_pos) {
                        current_line.push(canvas_pos);
                    }
                }
            } else if !current_line.is_empty() {
                self.lines.push(vec![]);
            }

            for line in &self.lines {
                if line.len() >= 2 {
                    region.add_paint_cmd(PaintCmd::Line {
                        points: line.iter().map(|p| canvas_corner + *p).collect(),
                        color: LIGHT_GRAY,
                        width: 2.0,
                    });
                }
            }

            // Frame it:
            region.add_paint_cmd(PaintCmd::Rect {
                rect: region.desired_rect,
                corner_radius: 0.0,
                fill_color: None,
                outline: Some(Outline::new(1.0, WHITE)),
            });
        });
    }
}

const LOREM_IPSUM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.

Curabitur pretium tincidunt lacus. Nulla gravida orci a odio. Nullam varius, turpis et commodo pharetra, est eros bibendum elit, nec luctus magna felis sollicitudin mauris. Integer in mauris eu nibh euismod gravida. Duis ac tellus et risus vulputate vehicula. Donec lobortis risus a elit. Etiam tempor. Ut ullamcorper, ligula eu tempor congue, eros est euismod turpis, id tincidunt sapien risus a quam. Maecenas fermentum consequat mi. Donec fermentum. Pellentesque malesuada nulla a mi. Duis sapien sem, aliquet nec, commodo eget, consequat quis, neque. Aliquam faucibus, elit ut dictum aliquet, felis nisl adipiscing sapien, sed malesuada diam lacus eget erat. Cras mollis scelerisque nunc. Nullam arcu. Aliquam consequat. Curabitur augue lorem, dapibus quis, laoreet et, pretium ac, nisi. Aenean magna nisl, mollis quis, molestie eu, feugiat in, orci. In hac habitasse platea dictumst.";
