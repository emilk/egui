#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

use std::sync::Arc;

use eframe::egui;
use eframe::egui::{Rangef, Shape, UiKind};
use egui_extras::Column;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Stack Frame Demo",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::<MyApp>::default())
        }),
    )
}

#[derive(Default)]
struct MyApp {
    show_settings: bool,
    show_inspection: bool,
    show_memory: bool,
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.all_styles_mut(|style| style.interaction.tooltip_delay = 0.0);

        egui::Panel::left("side_panel_left").show_inside(ui, |ui| {
            ui.heading("Information");
            ui.label(
                "This is a demo/test environment of the `UiStack` feature. The tables display \
                the UI stack in various contexts. You can hover on the IDs to display the \
                corresponding origin/`max_rect`.\n\n\
                The \"Full span test\" labels showcase an implementation of full-span \
                highlighting. Hover to see them in action!",
            );
            ui.add_space(10.0);
            ui.checkbox(&mut self.show_settings, "🔧 Settings");
            ui.checkbox(&mut self.show_inspection, "🔍 Inspection");
            ui.checkbox(&mut self.show_memory, "📝 Memory");
            ui.add_space(10.0);
            if ui.button("Reset egui memory").clicked() {
                ui.memory_mut(|mem| *mem = Default::default());
            }
            ui.add_space(20.0);

            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                stack_ui(ui);

                // full span test
                ui.add_space(20.0);
                full_span_widget(ui, false);

                // nested frames test
                ui.add_space(20.0);
                egui::Frame::new()
                    .stroke(ui.visuals().noninteractive().bg_stroke)
                    .inner_margin(4)
                    .outer_margin(4)
                    .show(ui, |ui| {
                        full_span_widget(ui, false);
                        stack_ui(ui);

                        egui::Frame::new()
                            .stroke(ui.visuals().noninteractive().bg_stroke)
                            .inner_margin(8)
                            .outer_margin(6)
                            .show(ui, |ui| {
                                full_span_widget(ui, false);
                                stack_ui(ui);
                            });
                    });
            });
        });

        egui::Panel::right("side_panel_right").show_inside(ui, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                stack_ui(ui);

                // full span test
                ui.add_space(20.0);
                full_span_widget(ui, false);
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                ui.label("stack here:");
                stack_ui(ui);

                // full span test
                ui.add_space(20.0);
                full_span_widget(ui, false);

                // tooltip test
                ui.add_space(20.0);
                ui.label("Hover me").on_hover_ui(|ui| {
                    full_span_widget(ui, true);
                    ui.add_space(20.0);
                    stack_ui(ui);
                });

                // combobox test
                ui.add_space(20.0);
                egui::ComboBox::from_id_salt("combo_box")
                    .selected_text("click me")
                    .show_ui(ui, |ui| {
                        full_span_widget(ui, true);
                        ui.add_space(20.0);
                        stack_ui(ui);
                    });

                // Ui nesting test
                ui.add_space(20.0);
                ui.label("UI nesting test:");
                egui::Frame::new()
                    .stroke(ui.visuals().noninteractive().bg_stroke)
                    .inner_margin(4)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.scope(stack_ui);
                            });
                        });
                    });

                // table test
                let mut cell_stack = None;
                ui.add_space(20.0);
                ui.label("Table test:");

                egui_extras::TableBuilder::new(ui)
                    .vscroll(false)
                    .column(Column::auto())
                    .column(Column::auto())
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.strong("column 1");
                        });
                        header.col(|ui| {
                            ui.strong("column 2");
                        });
                    })
                    .body(|mut body| {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                full_span_widget(ui, false);
                            });
                            row.col(|ui| {
                                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                                ui.label("See stack below");
                                cell_stack = Some(Arc::clone(ui.stack()));
                            });
                        });
                    });

                if let Some(cell_stack) = cell_stack {
                    ui.label("Cell's stack:");
                    stack_ui_impl(ui, &cell_stack);
                }
            });
        });

        egui::Panel::bottom("bottom_panel")
            .resizable(true)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        stack_ui(ui);

                        // full span test
                        ui.add_space(20.0);
                        full_span_widget(ui, false);
                    });
            });

        egui::Window::new("Window")
            .pivot(egui::Align2::RIGHT_TOP)
            .show(ui.ctx(), |ui| {
                full_span_widget(ui, false);
                ui.add_space(20.0);
                stack_ui(ui);
            });

        let ctx = ui.ctx().clone();
        egui::Window::new("🔧 Settings")
            .open(&mut self.show_settings)
            .vscroll(true)
            .show(&ctx, |ui| {
                ctx.settings_ui(ui);
            });

        egui::Window::new("🔍 Inspection")
            .open(&mut self.show_inspection)
            .vscroll(true)
            .show(&ctx, |ui| {
                ctx.inspection_ui(ui);
            });

        egui::Window::new("📝 Memory")
            .open(&mut self.show_memory)
            .resizable(false)
            .show(&ctx, |ui| {
                ctx.memory_ui(ui);
            });
    }
}

/// Demo of a widget that highlights its background all the way to the edge of its container when
/// hovered.
fn full_span_widget(ui: &mut egui::Ui, permanent: bool) {
    let bg_shape_idx = ui.painter().add(Shape::Noop);
    let response = ui.label("Full span test");
    let ui_stack = ui.stack();

    let rect = egui::Rect::from_x_y_ranges(
        full_span_horizontal_range(ui_stack),
        response.rect.y_range(),
    );

    if permanent || response.hovered() {
        ui.painter().set(
            bg_shape_idx,
            Shape::rect_filled(rect, 0.0, ui.visuals().selection.bg_fill),
        );
    }
}

/// Find the horizontal range of the enclosing container.
fn full_span_horizontal_range(ui_stack: &egui::UiStack) -> Rangef {
    for node in ui_stack.iter() {
        if node.has_visible_frame()
            || node.is_panel_ui()
            || node.is_root_ui()
            || node.kind() == Some(UiKind::TableCell)
        {
            return (node.max_rect + node.frame().inner_margin).x_range();
        }
    }

    // should never happen
    Rangef::EVERYTHING
}

fn stack_ui(ui: &mut egui::Ui) {
    let ui_stack = Arc::clone(ui.stack());
    ui.scope(|ui| {
        stack_ui_impl(ui, &ui_stack);
    });
}

fn stack_ui_impl(ui: &mut egui::Ui, stack: &egui::UiStack) {
    egui::Frame::new()
        .stroke(ui.style().noninteractive().fg_stroke)
        .inner_margin(4)
        .show(ui, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

            egui_extras::TableBuilder::new(ui)
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("id");
                    });
                    header.col(|ui| {
                        ui.strong("kind");
                    });
                    header.col(|ui| {
                        ui.strong("stroke");
                    });
                    header.col(|ui| {
                        ui.strong("inner");
                    });
                    header.col(|ui| {
                        ui.strong("outer");
                    });
                    header.col(|ui| {
                        ui.strong("direction");
                    });
                })
                .body(|mut body| {
                    for node in stack.iter() {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                if ui.label(format!("{:?}", node.id)).hovered() {
                                    ui.debug_painter().debug_rect(
                                        node.max_rect,
                                        egui::Color32::GREEN,
                                        "max",
                                    );
                                    ui.debug_painter().circle_filled(
                                        node.min_rect.min,
                                        2.0,
                                        egui::Color32::RED,
                                    );
                                }
                            });
                            row.col(|ui| {
                                let s = if let Some(kind) = node.kind() {
                                    format!("{kind:?}")
                                } else {
                                    "-".to_owned()
                                };

                                ui.label(s);
                            });
                            row.col(|ui| {
                                let frame = node.frame();
                                if frame.stroke == egui::Stroke::NONE {
                                    ui.label("-");
                                } else {
                                    let mut layout_job = egui::text::LayoutJob::default();
                                    layout_job.append(
                                        "⬛ ",
                                        0.0,
                                        egui::TextFormat::simple(
                                            egui::TextStyle::Body.resolve(ui.style()),
                                            frame.stroke.color,
                                        ),
                                    );
                                    layout_job.append(
                                        format!("{}px", frame.stroke.width).as_str(),
                                        0.0,
                                        egui::TextFormat::simple(
                                            egui::TextStyle::Body.resolve(ui.style()),
                                            ui.style().visuals.text_color(),
                                        ),
                                    );
                                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                                    ui.label(layout_job);
                                }
                            });
                            row.col(|ui| {
                                ui.label(print_margin(&node.frame().inner_margin));
                            });
                            row.col(|ui| {
                                ui.label(print_margin(&node.frame().outer_margin));
                            });
                            row.col(|ui| {
                                ui.label(format!("{:?}", node.layout_direction));
                            });
                        });
                    }
                });
        });
}

fn print_margin(margin: &egui::Margin) -> String {
    if margin.is_same() {
        format!("{}px", margin.left)
    } else {
        let s1 = if margin.left == margin.right {
            format!("H: {}px", margin.left)
        } else {
            format!("L: {}px R: {}px", margin.left, margin.right)
        };
        let s2 = if margin.top == margin.bottom {
            format!("V: {}px", margin.top)
        } else {
            format!("T: {}px B: {}px", margin.top, margin.bottom)
        };
        format!("{s1} / {s2}")
    }
}
