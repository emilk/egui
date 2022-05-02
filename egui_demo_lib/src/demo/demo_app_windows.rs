use egui::{Context, ScrollArea, Ui};
use std::collections::BTreeSet;

use super::About;
use super::Demo;
use super::View;
use crate::is_mobile;

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
struct Demos {
    #[cfg_attr(feature = "serde", serde(skip))]
    demos: Vec<Box<dyn Demo>>,

    open: BTreeSet<String>,
}

impl Default for Demos {
    fn default() -> Self {
        Self::from_demos(vec![
            Box::new(super::paint_bezier::PaintBezier::default()),
            Box::new(super::code_editor::CodeEditor::default()),
            Box::new(super::code_example::CodeExample::default()),
            Box::new(super::context_menu::ContextMenus::default()),
            Box::new(super::dancing_strings::DancingStrings::default()),
            Box::new(super::drag_and_drop::DragAndDropDemo::default()),
            Box::new(super::font_book::FontBook::default()),
            Box::new(super::MiscDemoWindow::default()),
            Box::new(super::multi_touch::MultiTouch::default()),
            Box::new(super::painting::Painting::default()),
            Box::new(super::plot_demo::PlotDemo::default()),
            Box::new(super::scrolling::Scrolling::default()),
            Box::new(super::sliders::Sliders::default()),
            Box::new(super::strip_demo::StripDemo::default()),
            Box::new(super::table_demo::TableDemo::default()),
            Box::new(super::text_edit::TextEdit::default()),
            Box::new(super::widget_gallery::WidgetGallery::default()),
            Box::new(super::window_options::WindowOptions::default()),
            Box::new(super::tests::WindowResizeTest::default()),
            Box::new(super::window_with_panels::WindowWithPanels::default()),
        ])
    }
}

impl Demos {
    pub fn from_demos(demos: Vec<Box<dyn Demo>>) -> Self {
        let mut open = BTreeSet::new();
        open.insert(
            super::widget_gallery::WidgetGallery::default()
                .name()
                .to_owned(),
        );

        Self { demos, open }
    }

    pub fn checkboxes(&mut self, ui: &mut Ui) {
        let Self { demos, open } = self;
        for demo in demos {
            let mut is_open = open.contains(demo.name());
            ui.toggle_value(&mut is_open, demo.name());
            set_open(open, demo.name(), is_open);
        }
    }

    pub fn windows(&mut self, ctx: &Context) {
        let Self { demos, open } = self;
        for demo in demos {
            let mut is_open = open.contains(demo.name());
            demo.show(ctx, &mut is_open);
            set_open(open, demo.name(), is_open);
        }
    }
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
struct Tests {
    #[cfg_attr(feature = "serde", serde(skip))]
    demos: Vec<Box<dyn Demo>>,

    open: BTreeSet<String>,
}

impl Default for Tests {
    fn default() -> Self {
        Self::from_demos(vec![
            Box::new(super::tests::CursorTest::default()),
            Box::new(super::tests::IdTest::default()),
            Box::new(super::tests::InputTest::default()),
            Box::new(super::layout_test::LayoutTest::default()),
            Box::new(super::tests::ManualLayoutTest::default()),
            Box::new(super::tests::TableTest::default()),
        ])
    }
}

impl Tests {
    pub fn from_demos(demos: Vec<Box<dyn Demo>>) -> Self {
        let mut open = BTreeSet::new();
        open.insert(
            super::widget_gallery::WidgetGallery::default()
                .name()
                .to_owned(),
        );

        Self { demos, open }
    }

    pub fn checkboxes(&mut self, ui: &mut Ui) {
        let Self { demos, open } = self;
        for demo in demos {
            let mut is_open = open.contains(demo.name());
            ui.toggle_value(&mut is_open, demo.name());
            set_open(open, demo.name(), is_open);
        }
    }

    pub fn windows(&mut self, ctx: &Context) {
        let Self { demos, open } = self;
        for demo in demos {
            let mut is_open = open.contains(demo.name());
            demo.show(ctx, &mut is_open);
            set_open(open, demo.name(), is_open);
        }
    }
}

// ----------------------------------------------------------------------------

fn set_open(open: &mut BTreeSet<String>, key: &'static str, is_open: bool) {
    if is_open {
        if !open.contains(key) {
            open.insert(key.to_owned());
        }
    } else {
        open.remove(key);
    }
}

// ----------------------------------------------------------------------------

/// A menu bar in which you can select different demo windows to show.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct DemoWindows {
    about_is_open: bool,
    about: About,
    demos: Demos,
    tests: Tests,
}

impl Default for DemoWindows {
    fn default() -> Self {
        Self {
            about_is_open: true,
            about: Default::default(),
            demos: Default::default(),
            tests: Default::default(),
        }
    }
}

impl DemoWindows {
    /// Show the app ui (menu bar and windows).
    pub fn ui(&mut self, ctx: &Context) {
        if is_mobile(ctx) {
            self.mobile_ui(ctx);
        } else {
            self.desktop_ui(ctx);
        }
    }

    fn mobile_ui(&mut self, ctx: &Context) {
        if self.about_is_open {
            egui::CentralPanel::default().show(ctx, |_ui| {}); // just to paint a background for the windows to be on top of. Needed on web because of https://github.com/emilk/egui/issues/1548

            let screen_size = ctx.input().screen_rect.size();
            let default_width = (screen_size.x - 20.0).min(400.0);

            let mut close = false;
            egui::Window::new(self.about.name())
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .default_width(default_width)
                .default_height(ctx.available_rect().height() - 46.0)
                .vscroll(true)
                .open(&mut self.about_is_open)
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    self.about.ui(ui);
                    ui.add_space(12.0);
                    ui.vertical_centered_justified(|ui| {
                        if ui
                            .button(egui::RichText::new("Continue to the demo!").size(24.0))
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });
            self.about_is_open &= !close;
        } else {
            self.mobile_top_bar(ctx);
            self.show_windows(ctx);
        }
    }

    fn mobile_top_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                let font_size = 20.0;

                ui.menu_button(egui::RichText::new("⏷ demos").size(font_size), |ui| {
                    ui.set_style(ui.ctx().style()); // ignore the "menu" style set by `menu_button`.
                    self.demo_list_ui(ui);
                    if ui.ui_contains_pointer() && ui.input().pointer.any_click() {
                        ui.close_menu();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(), |ui| {
                    use egui::special_emojis::{GITHUB, TWITTER};
                    ui.hyperlink_to(
                        egui::RichText::new(TWITTER).size(font_size),
                        "https://twitter.com/ernerfeldt",
                    );
                    ui.hyperlink_to(
                        egui::RichText::new(GITHUB).size(font_size),
                        "https://github.com/emilk/egui",
                    );
                });
            });
        });
    }

    fn desktop_ui(&mut self, ctx: &Context) {
        egui::SidePanel::right("egui_demo_panel")
            .resizable(false)
            .default_width(145.0)
            .show(ctx, |ui| {
                egui::trace!(ui);
                ui.vertical_centered(|ui| {
                    ui.heading("✒ egui demos");
                });

                ui.separator();

                use egui::special_emojis::{GITHUB, TWITTER};
                ui.hyperlink_to(
                    format!("{} egui on GitHub", GITHUB),
                    "https://github.com/emilk/egui",
                );
                ui.hyperlink_to(
                    format!("{} @ernerfeldt", TWITTER),
                    "https://twitter.com/ernerfeldt",
                );

                ui.separator();

                self.demo_list_ui(ui);
            });

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                file_menu_button(ui);
            });
        });

        self.show_windows(ctx);
    }

    /// Show the open windows.
    fn show_windows(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |_ui| {}); // just to paint a background for the windows to be on top of. Needed on web because of https://github.com/emilk/egui/issues/1548
        self.about.show(ctx, &mut self.about_is_open);
        self.demos.windows(ctx);
        self.tests.windows(ctx);
    }

    fn demo_list_ui(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                ui.toggle_value(&mut self.about_is_open, self.about.name());

                ui.separator();
                self.demos.checkboxes(ui);
                ui.separator();
                self.tests.checkboxes(ui);
                ui.separator();

                if ui.button("Organize windows").clicked() {
                    ui.ctx().memory().reset_areas();
                }
            });
        });
    }
}

// ----------------------------------------------------------------------------

fn file_menu_button(ui: &mut Ui) {
    ui.menu_button("File", |ui| {
        if ui.button("Organize windows").clicked() {
            ui.ctx().memory().reset_areas();
            ui.close_menu();
        }
        if ui
            .button("Reset egui memory")
            .on_hover_text("Forget scroll, positions, sizes etc")
            .clicked()
        {
            *ui.ctx().memory() = Default::default();
            ui.close_menu();
        }
    });
}
