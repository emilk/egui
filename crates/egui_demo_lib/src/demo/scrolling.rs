use egui::*;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
enum ScrollDemo {
    ScrollTo,
    ManyLines,
    LargeCanvas,
    StickToEnd,
    Bidirectional,
}

impl Default for ScrollDemo {
    fn default() -> Self {
        Self::ScrollTo
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
#[derive(Default, PartialEq)]
pub struct Scrolling {
    demo: ScrollDemo,
    scroll_to: ScrollTo,
    scroll_stick_to: ScrollStickTo,
}

impl super::Demo for Scrolling {
    fn name(&self) -> &'static str {
        "↕ Scrolling"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .show(ctx, |ui| {
                use super::View as _;
                self.ui(ui);
            });
    }
}

impl super::View for Scrolling {
    fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.demo, ScrollDemo::ScrollTo, "Scroll to");
            ui.selectable_value(
                &mut self.demo,
                ScrollDemo::ManyLines,
                "Scroll a lot of lines",
            );
            ui.selectable_value(
                &mut self.demo,
                ScrollDemo::LargeCanvas,
                "Scroll a large canvas",
            );
            ui.selectable_value(&mut self.demo, ScrollDemo::StickToEnd, "Stick to end");
            ui.selectable_value(&mut self.demo, ScrollDemo::Bidirectional, "Bidirectional");
        });
        ui.separator();
        match self.demo {
            ScrollDemo::ScrollTo => {
                self.scroll_to.ui(ui);
            }
            ScrollDemo::ManyLines => {
                huge_content_lines(ui);
            }
            ScrollDemo::LargeCanvas => {
                huge_content_painter(ui);
            }
            ScrollDemo::StickToEnd => {
                self.scroll_stick_to.ui(ui);
            }
            ScrollDemo::Bidirectional => {
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.style_mut().wrap = Some(false);
                    for _ in 0..100 {
                        ui.label(crate::LOREM_IPSUM);
                    }
                });
            }
        }
    }
}

fn huge_content_lines(ui: &mut egui::Ui) {
    ui.label(
        "A lot of rows, but only the visible ones are layed out, so performance is still good:",
    );
    ui.add_space(4.0);

    let text_style = TextStyle::Body;
    let row_height = ui.text_style_height(&text_style);
    let num_rows = 10_000;
    ScrollArea::vertical().auto_shrink([false; 2]).show_rows(
        ui,
        row_height,
        num_rows,
        |ui, row_range| {
            for row in row_range {
                let text = format!("This is row {}/{}", row + 1, num_rows);
                ui.label(text);
            }
        },
    );
}

fn huge_content_painter(ui: &mut egui::Ui) {
    // This is similar to the other demo, but is fully manual, for when you want to do custom painting.
    ui.label("A lot of rows, but only the visible ones are painted, so performance is still good:");
    ui.add_space(4.0);

    let font_id = TextStyle::Body.resolve(ui.style());
    let row_height = ui.fonts().row_height(&font_id) + ui.spacing().item_spacing.y;
    let num_rows = 10_000;

    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show_viewport(ui, |ui, viewport| {
            ui.set_height(row_height * num_rows as f32);

            let first_item = (viewport.min.y / row_height).floor().at_least(0.0) as usize;
            let last_item = (viewport.max.y / row_height).ceil() as usize + 1;
            let last_item = last_item.at_most(num_rows);

            let mut used_rect = Rect::NOTHING;

            for i in first_item..last_item {
                let indentation = (i % 100) as f32;
                let x = ui.min_rect().left() + indentation;
                let y = ui.min_rect().top() + i as f32 * row_height;
                let text = format!(
                    "This is row {}/{}, indented by {} pixels",
                    i + 1,
                    num_rows,
                    indentation
                );
                let text_rect = ui.painter().text(
                    pos2(x, y),
                    Align2::LEFT_TOP,
                    text,
                    font_id.clone(),
                    ui.visuals().text_color(),
                );
                used_rect = used_rect.union(text_rect);
            }

            ui.allocate_rect(used_rect, Sense::hover()); // make sure it is visible!
        });
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
#[derive(PartialEq)]
struct ScrollTo {
    track_item: usize,
    tack_item_align: Option<Align>,
    offset: f32,
}

impl Default for ScrollTo {
    fn default() -> Self {
        Self {
            track_item: 25,
            tack_item_align: Some(Align::Center),
            offset: 0.0,
        }
    }
}

impl super::View for ScrollTo {
    fn ui(&mut self, ui: &mut Ui) {
        ui.label("This shows how you can scroll to a specific item or pixel offset");

        let mut track_item = false;
        let mut go_to_scroll_offset = false;
        let mut scroll_top = false;
        let mut scroll_bottom = false;

        ui.horizontal(|ui| {
            ui.label("Scroll to a specific item index:");
            track_item |= ui
                .add(Slider::new(&mut self.track_item, 1..=50).text("Track Item"))
                .dragged();
        });

        ui.horizontal(|ui| {
            ui.label("Item align:");
            track_item |= ui
                .radio_value(&mut self.tack_item_align, Some(Align::Min), "Top")
                .clicked();
            track_item |= ui
                .radio_value(&mut self.tack_item_align, Some(Align::Center), "Center")
                .clicked();
            track_item |= ui
                .radio_value(&mut self.tack_item_align, Some(Align::Max), "Bottom")
                .clicked();
            track_item |= ui
                .radio_value(&mut self.tack_item_align, None, "None (Bring into view)")
                .clicked();
        });

        ui.horizontal(|ui| {
            ui.label("Scroll to a specific offset:");
            go_to_scroll_offset |= ui
                .add(DragValue::new(&mut self.offset).speed(1.0).suffix("px"))
                .dragged();
        });

        ui.horizontal(|ui| {
            scroll_top |= ui.button("Scroll to top").clicked();
            scroll_bottom |= ui.button("Scroll to bottom").clicked();
        });

        let mut scroll_area = ScrollArea::vertical()
            .max_height(200.0)
            .auto_shrink([false; 2]);
        if go_to_scroll_offset {
            scroll_area = scroll_area.vertical_scroll_offset(self.offset);
        }

        ui.separator();
        let (current_scroll, max_scroll) = scroll_area
            .show(ui, |ui| {
                if scroll_top {
                    ui.scroll_to_cursor(Some(Align::TOP));
                }
                ui.vertical(|ui| {
                    for item in 1..=50 {
                        if track_item && item == self.track_item {
                            let response =
                                ui.colored_label(Color32::YELLOW, format!("This is item {}", item));
                            response.scroll_to_me(self.tack_item_align);
                        } else {
                            ui.label(format!("This is item {}", item));
                        }
                    }
                });

                if scroll_bottom {
                    ui.scroll_to_cursor(Some(Align::BOTTOM));
                }

                let margin = ui.visuals().clip_rect_margin;

                let current_scroll = ui.clip_rect().top() - ui.min_rect().top() + margin;
                let max_scroll = ui.min_rect().height() - ui.clip_rect().height() + 2.0 * margin;
                (current_scroll, max_scroll)
            })
            .inner;
        ui.separator();

        ui.label(format!(
            "Scroll offset: {:.0}/{:.0} px",
            current_scroll, max_scroll
        ));

        ui.separator();
        ui.vertical_centered(|ui| {
            egui::reset_button(ui, self);
            ui.add(crate::egui_github_link_file!());
        });
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
#[derive(Default, PartialEq)]
struct ScrollStickTo {
    n_items: usize,
}

impl super::View for ScrollStickTo {
    fn ui(&mut self, ui: &mut Ui) {
        ui.label("Rows enter from the bottom, we want the scroll handle to start and stay at bottom unless moved");

        ui.add_space(4.0);

        let text_style = TextStyle::Body;
        let row_height = ui.text_style_height(&text_style);
        ScrollArea::vertical().stick_to_bottom(true).show_rows(
            ui,
            row_height,
            self.n_items,
            |ui, row_range| {
                for row in row_range {
                    let text = format!("This is row {}", row + 1);
                    ui.label(text);
                }
            },
        );

        self.n_items += 1;
        ui.ctx().request_repaint();
    }
}
