use egui::{color::*, *};

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
enum ScrollDemo {
    ScrollTo,
    ManyLines,
    LargeCanvas,
}

impl Default for ScrollDemo {
    fn default() -> Self {
        Self::ScrollTo
    }
}

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
#[derive(Default, PartialEq)]
pub struct Scrolling {
    demo: ScrollDemo,
    scroll_to: ScrollTo,
}

impl super::Demo for Scrolling {
    fn name(&self) -> &'static str {
        "↕ Scrolling"
    }

    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .show(ctx, |ui| {
                use super::View;
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
        }
    }
}

fn huge_content_lines(ui: &mut egui::Ui) {
    ui.label(
        "A lot of rows, but only the visible ones are layed out, so performance is still good:",
    );
    ui.add_space(4.0);

    let text_style = TextStyle::Body;
    let row_height = ui.fonts()[text_style].row_height();
    let num_rows = 10_000;
    ScrollArea::auto_sized().show_rows(ui, row_height, num_rows, |ui, row_range| {
        for row in row_range {
            let text = format!("This is row {}/{}", row + 1, num_rows);
            ui.label(text);
        }
    });
}

fn huge_content_painter(ui: &mut egui::Ui) {
    // This is similar to the other demo, but is fully manual, for when you want to do custom painting.
    ui.label("A lot of rows, but only the visible ones are painted, so performance is still good:");
    ui.add_space(4.0);

    let text_style = TextStyle::Body;
    let row_height = ui.fonts()[text_style].row_height() + ui.spacing().item_spacing.y;
    let num_rows = 10_000;

    ScrollArea::auto_sized().show_viewport(ui, |ui, viewport| {
        ui.set_height(row_height * num_rows as f32);

        let first_item = (viewport.min.y / row_height).floor().at_least(0.0) as usize;
        let last_item = (viewport.max.y / row_height).ceil() as usize + 1;
        let last_item = last_item.at_most(num_rows);

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
            ui.painter().text(
                pos2(x, y),
                Align2::LEFT_TOP,
                text,
                text_style,
                ui.visuals().text_color(),
            );
        }
    });
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
#[derive(PartialEq)]
struct ScrollTo {
    track_item: usize,
    tack_item_align: Align,
    offset: f32,
}

impl Default for ScrollTo {
    fn default() -> Self {
        Self {
            track_item: 25,
            tack_item_align: Align::Center,
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
                .radio_value(&mut self.tack_item_align, Align::Min, "Top")
                .clicked();
            track_item |= ui
                .radio_value(&mut self.tack_item_align, Align::Center, "Center")
                .clicked();
            track_item |= ui
                .radio_value(&mut self.tack_item_align, Align::Max, "Bottom")
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

        let mut scroll_area = ScrollArea::from_max_height(200.0);
        if go_to_scroll_offset {
            scroll_area = scroll_area.scroll_offset(self.offset);
        }

        ui.separator();
        let (current_scroll, max_scroll) = scroll_area.show(ui, |ui| {
            if scroll_top {
                ui.scroll_to_cursor(Align::TOP);
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
                ui.scroll_to_cursor(Align::BOTTOM);
            }

            let margin = ui.visuals().clip_rect_margin;

            let current_scroll = ui.clip_rect().top() - ui.min_rect().top() + margin;
            let max_scroll = ui.min_rect().height() - ui.clip_rect().height() + 2.0 * margin;
            (current_scroll, max_scroll)
        });
        ui.separator();

        ui.label(format!(
            "Scroll offset: {:.0}/{:.0} px",
            current_scroll, max_scroll
        ));

        ui.separator();
        ui.vertical_centered(|ui| {
            egui::reset_button(ui, self);
            ui.add(crate::__egui_github_link_file!());
        });
    }
}
