#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

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
    settings: bool,
    inspection: bool,
    memory: bool,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.style_mut(|style| style.interaction.tooltip_delay = 0.0);

        egui::SidePanel::left("side_panel_left").show(ctx, |ui| {
            ui.heading("Information");
            ui.label(
                "This is a demo/test environment of the `UiStack` feature. The tables display \
                the UI stack in various contexts. You can hover on the IDs to display the \
                corresponding origin/`max_rect`.\n\n\
                The \"Full span test\" labels showcase an implementation of full-span \
                highlighting. Hover to see them in action!",
            );
            ui.add_space(10.0);
            ui.checkbox(&mut self.settings, "🔧 Settings");
            ui.checkbox(&mut self.inspection, "🔍 Inspection");
            ui.checkbox(&mut self.memory, "📝 Memory");
            ui.add_space(10.0);
            if ui.button("Reset egui memory").clicked() {
                ctx.memory_mut(|mem| *mem = Default::default());
            }
            ui.add_space(20.0);

            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                stack_ui(ui);

                // full span test
                ui.add_space(20.0);
                full_span_widget(ui, false);

                // nested frames test
                ui.add_space(20.0);
                egui::Frame {
                    stroke: ui.visuals().noninteractive().bg_stroke,
                    inner_margin: egui::Margin::same(4.0),
                    outer_margin: egui::Margin::same(4.0),
                    ..Default::default()
                }
                .show(ui, |ui| {
                    full_span_widget(ui, false);
                    stack_ui(ui);

                    egui::Frame {
                        stroke: ui.visuals().noninteractive().bg_stroke,
                        inner_margin: egui::Margin::same(8.0),
                        outer_margin: egui::Margin::same(6.0),
                        ..Default::default()
                    }
                    .show(ui, |ui| {
                        full_span_widget(ui, false);
                        stack_ui(ui);
                    });
                });
            });
        });

        egui::SidePanel::right("side_panel_right").show(ctx, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                stack_ui(ui);

                // full span test
                ui.add_space(20.0);
                full_span_widget(ui, false);
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        stack_ui(ui);

                        // full span test
                        ui.add_space(20.0);
                        full_span_widget(ui, false);
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
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
                egui::ComboBox::from_id_source("combo_box")
                    .selected_text("click me")
                    .show_ui(ui, |ui| {
                        full_span_widget(ui, true);
                        ui.add_space(20.0);
                        stack_ui(ui);
                    });

                // Ui nesting test
                ui.add_space(20.0);
                ui.label("UI nesting test:");
                egui::Frame {
                    stroke: ui.visuals().noninteractive().bg_stroke,
                    inner_margin: egui::Margin::same(4.0),
                    ..Default::default()
                }
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
                                cell_stack = Some(ui.stack().clone());
                            });
                        });
                    });

                if let Some(cell_stack) = cell_stack {
                    ui.label("Cell's stack:");
                    stack_ui_impl(ui, &cell_stack);
                }
            });
        });

        egui::Window::new("Window")
            .pivot(egui::Align2::RIGHT_TOP)
            .show(ctx, |ui| {
                full_span_widget(ui, false);
                ui.add_space(20.0);
                stack_ui(ui);
            });

        egui::Window::new("🔧 Settings")
            .open(&mut self.settings)
            .vscroll(true)
            .show(ctx, |ui| {
                ctx.settings_ui(ui);
            });

        egui::Window::new("🔍 Inspection")
            .open(&mut self.inspection)
            .vscroll(true)
            .show(ctx, |ui| {
                ctx.inspection_ui(ui);
            });

        egui::Window::new("📝 Memory")
            .open(&mut self.memory)
            .resizable(false)
            .show(ctx, |ui| {
                ctx.memory_ui(ui);
            });
    }
}
