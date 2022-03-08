use super::Demo;
use egui::{Context, ScrollArea, Ui};
use std::collections::BTreeSet;

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
            ui.checkbox(&mut is_open, demo.name());
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
            ui.checkbox(&mut is_open, demo.name());
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
#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct DemoWindows {
    demos: Demos,
    tests: Tests,
}

impl DemoWindows {
    /// Show the app ui (menu bar and windows).
    /// `sidebar_ui` can be used to optionally show some things in the sidebar
    pub fn ui(&mut self, ctx: &Context) {
        let Self { demos, tests } = self;

        egui::SidePanel::right("egui_demo_panel")
            .min_width(150.0)
            .default_width(180.0)
            .show(ctx, |ui| {
                egui::trace!(ui);
                ui.vertical_centered(|ui| {
                    ui.heading("✒ egui demos");
                });

                ui.separator();

                ScrollArea::vertical().show(ui, |ui| {
                    use egui::special_emojis::{GITHUB, OS_APPLE, OS_LINUX, OS_WINDOWS};

                    ui.vertical_centered(|ui| {
                        ui.label("egui is an immediate mode GUI library written in Rust.");

                        ui.label(format!(
                            "egui runs on the web, or natively on {}{}{}",
                            OS_APPLE, OS_LINUX, OS_WINDOWS,
                        ));

                        ui.hyperlink_to(
                            format!("{} egui home page", GITHUB),
                            "https://github.com/emilk/egui",
                        );
                    });

                    ui.separator();
                    demos.checkboxes(ui);
                    ui.separator();
                    tests.checkboxes(ui);
                    ui.separator();

                    ui.vertical_centered(|ui| {
                        if ui.button("Organize windows").clicked() {
                            ui.ctx().memory().reset_areas();
                        }
                    });
                });
            });

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            show_menu_bar(ui);
        });

        {
            let mut fill = ctx.style().visuals.extreme_bg_color;
            if !cfg!(target_arch = "wasm32") {
                // Native: WrapApp uses a transparent window, so let's show that off:
                // NOTE: the OS compositor assumes "normal" blending, so we need to hack it:
                let [r, g, b, _] = fill.to_array();
                fill = egui::Color32::from_rgba_premultiplied(r, g, b, 180);
            }
            let frame = egui::Frame::none().fill(fill);
            egui::CentralPanel::default().frame(frame).show(ctx, |_| {});
        }

        self.windows(ctx);
    }

    /// Show the open windows.
    fn windows(&mut self, ctx: &Context) {
        let Self { demos, tests } = self;

        demos.windows(ctx);
        tests.windows(ctx);
    }
}

// ----------------------------------------------------------------------------

fn show_menu_bar(ui: &mut Ui) {
    trace!(ui);
    use egui::*;

    menu::bar(ui, |ui| {
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
    });
}
