use std::collections::BTreeSet;

use egui::{Context, Modifiers, ScrollArea, Ui};

use super::About;
use crate::is_mobile;
use crate::Demo;
use crate::View;

// ----------------------------------------------------------------------------

struct DemoGroup {
    demos: Vec<Box<dyn Demo>>,
}

impl DemoGroup {
    pub fn new(demos: Vec<Box<dyn Demo>>) -> Self {
        Self { demos }
    }

    pub fn checkboxes(&mut self, ui: &mut Ui, open: &mut BTreeSet<String>) {
        let Self { demos } = self;
        for demo in demos {
            if demo.is_enabled(ui.ctx()) {
                let mut is_open = open.contains(demo.name());
                ui.toggle_value(&mut is_open, demo.name());
                set_open(open, demo.name(), is_open);
            }
        }
    }

    pub fn windows(&mut self, ctx: &Context, open: &mut BTreeSet<String>) {
        let Self { demos } = self;
        for demo in demos {
            let mut is_open = open.contains(demo.name());
            demo.show(ctx, &mut is_open);
            set_open(open, demo.name(), is_open);
        }
    }
}

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

pub struct DemoGroups {
    about: About,
    demos: DemoGroup,
    tests: DemoGroup,
}

impl Default for DemoGroups {
    fn default() -> Self {
        Self {
            about: About::default(),
            demos: DemoGroup::new(vec![
                Box::<super::paint_bezier::PaintBezier>::default(),
                Box::<super::code_editor::CodeEditor>::default(),
                Box::<super::code_example::CodeExample>::default(),
                Box::<super::context_menu::ContextMenus>::default(),
                Box::<super::dancing_strings::DancingStrings>::default(),
                Box::<super::drag_and_drop::DragAndDropDemo>::default(),
                Box::<super::extra_viewport::ExtraViewport>::default(),
                Box::<super::font_book::FontBook>::default(),
                Box::<super::frame_demo::FrameDemo>::default(),
                Box::<super::highlighting::Highlighting>::default(),
                Box::<super::interactive_container::InteractiveContainerDemo>::default(),
                Box::<super::MiscDemoWindow>::default(),
                Box::<super::modals::Modals>::default(),
                Box::<super::multi_touch::MultiTouch>::default(),
                Box::<super::painting::Painting>::default(),
                Box::<super::pan_zoom::PanZoom>::default(),
                Box::<super::panels::Panels>::default(),
                Box::<super::screenshot::Screenshot>::default(),
                Box::<super::scrolling::Scrolling>::default(),
                Box::<super::sliders::Sliders>::default(),
                Box::<super::strip_demo::StripDemo>::default(),
                Box::<super::table_demo::TableDemo>::default(),
                Box::<super::text_edit::TextEditDemo>::default(),
                Box::<super::text_layout::TextLayoutDemo>::default(),
                Box::<super::tooltips::Tooltips>::default(),
                Box::<super::undo_redo::UndoRedoDemo>::default(),
                Box::<super::widget_gallery::WidgetGallery>::default(),
                Box::<super::window_options::WindowOptions>::default(),
            ]),
            tests: DemoGroup::new(vec![
                Box::<super::tests::ClipboardTest>::default(),
                Box::<super::tests::CursorTest>::default(),
                Box::<super::tests::GridTest>::default(),
                Box::<super::tests::IdTest>::default(),
                Box::<super::tests::InputEventHistory>::default(),
                Box::<super::tests::InputTest>::default(),
                Box::<super::tests::LayoutTest>::default(),
                Box::<super::tests::ManualLayoutTest>::default(),
                Box::<super::tests::WindowResizeTest>::default(),
            ]),
        }
    }
}

impl DemoGroups {
    pub fn checkboxes(&mut self, ui: &mut Ui, open: &mut BTreeSet<String>) {
        let Self {
            about,
            demos,
            tests,
        } = self;

        {
            let mut is_open = open.contains(about.name());
            ui.toggle_value(&mut is_open, about.name());
            set_open(open, about.name(), is_open);
        }
        ui.separator();
        demos.checkboxes(ui, open);
        ui.separator();
        tests.checkboxes(ui, open);
    }

    pub fn windows(&mut self, ctx: &Context, open: &mut BTreeSet<String>) {
        let Self {
            about,
            demos,
            tests,
        } = self;
        {
            let mut is_open = open.contains(about.name());
            about.show(ctx, &mut is_open);
            set_open(open, about.name(), is_open);
        }
        demos.windows(ctx, open);
        tests.windows(ctx, open);
    }
}

// ----------------------------------------------------------------------------

/// A menu bar in which you can select different demo windows to show.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct DemoWindows {
    #[cfg_attr(feature = "serde", serde(skip))]
    groups: DemoGroups,

    open: BTreeSet<String>,
}

impl Default for DemoWindows {
    fn default() -> Self {
        let mut open = BTreeSet::new();

        // Explains egui very well
        set_open(&mut open, About::default().name(), true);

        // Explains egui very well
        set_open(
            &mut open,
            super::code_example::CodeExample::default().name(),
            true,
        );

        // Shows off the features
        set_open(
            &mut open,
            super::widget_gallery::WidgetGallery::default().name(),
            true,
        );

        Self {
            groups: Default::default(),
            open,
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

    fn about_is_open(&self) -> bool {
        self.open.contains(About::default().name())
    }

    fn mobile_ui(&mut self, ctx: &Context) {
        if self.about_is_open() {
            let mut close = false;
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        self.groups.about.ui(ui);
                        ui.add_space(12.0);
                        ui.vertical_centered_justified(|ui| {
                            if ui
                                .button(egui::RichText::new("Continue to the demo!").size(20.0))
                                .clicked()
                            {
                                close = true;
                            }
                        });
                    });
            });
            if close {
                set_open(&mut self.open, About::default().name(), false);
            }
        } else {
            self.mobile_top_bar(ctx);
            self.groups.windows(ctx, &mut self.open);
        }
    }

    fn mobile_top_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                let font_size = 16.5;

                ui.menu_button(egui::RichText::new("⏷ demos").size(font_size), |ui| {
                    ui.set_style(ui.ctx().style()); // ignore the "menu" style set by `menu_button`.
                    self.demo_list_ui(ui);
                    if ui.ui_contains_pointer() && ui.input(|i| i.pointer.any_click()) {
                        ui.close_menu();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
            .default_width(160.0)
            .min_width(160.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    ui.heading("✒ egui demos");
                });

                ui.separator();

                use egui::special_emojis::{GITHUB, TWITTER};
                ui.hyperlink_to(
                    format!("{GITHUB} egui on GitHub"),
                    "https://github.com/emilk/egui",
                );
                ui.hyperlink_to(
                    format!("{TWITTER} @ernerfeldt"),
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

        self.groups.windows(ctx, &mut self.open);
    }

    fn demo_list_ui(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                self.groups.checkboxes(ui, &mut self.open);
                ui.separator();
                if ui.button("Organize windows").clicked() {
                    ui.ctx().memory_mut(|mem| mem.reset_areas());
                }
            });
        });
    }
}

// ----------------------------------------------------------------------------

fn file_menu_button(ui: &mut Ui) {
    let organize_shortcut =
        egui::KeyboardShortcut::new(Modifiers::CTRL | Modifiers::SHIFT, egui::Key::O);
    let reset_shortcut =
        egui::KeyboardShortcut::new(Modifiers::CTRL | Modifiers::SHIFT, egui::Key::R);

    // NOTE: we must check the shortcuts OUTSIDE of the actual "File" menu,
    // or else they would only be checked if the "File" menu was actually open!

    if ui.input_mut(|i| i.consume_shortcut(&organize_shortcut)) {
        ui.ctx().memory_mut(|mem| mem.reset_areas());
    }

    if ui.input_mut(|i| i.consume_shortcut(&reset_shortcut)) {
        ui.ctx().memory_mut(|mem| *mem = Default::default());
    }

    ui.menu_button("File", |ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

        // On the web the browser controls the zoom
        #[cfg(not(target_arch = "wasm32"))]
        {
            egui::gui_zoom::zoom_menu_buttons(ui);
            ui.weak(format!(
                "Current zoom: {:.0}%",
                100.0 * ui.ctx().zoom_factor()
            ))
            .on_hover_text("The UI zoom level, on top of the operating system's default value");
            ui.separator();
        }

        if ui
            .add(
                egui::Button::new("Organize Windows")
                    .shortcut_text(ui.ctx().format_shortcut(&organize_shortcut)),
            )
            .clicked()
        {
            ui.ctx().memory_mut(|mem| mem.reset_areas());
            ui.close_menu();
        }

        if ui
            .add(
                egui::Button::new("Reset egui memory")
                    .shortcut_text(ui.ctx().format_shortcut(&reset_shortcut)),
            )
            .on_hover_text("Forget scroll, positions, sizes etc")
            .clicked()
        {
            ui.ctx().memory_mut(|mem| *mem = Default::default());
            ui.close_menu();
        }
    });
}

#[cfg(test)]
mod tests {
    use crate::{demo::demo_app_windows::DemoGroups, Demo};
    use egui::Vec2;
    use egui_kittest::kittest::Queryable;
    use egui_kittest::{Harness, SnapshotOptions};

    #[test]
    fn demos_should_match_snapshot() {
        let demos = DemoGroups::default().demos;

        let mut errors = Vec::new();

        for mut demo in demos.demos {
            // Widget Gallery needs to be customized (to set a specific date) and has its own test
            if demo.name() == crate::WidgetGallery::default().name() {
                continue;
            }

            // Remove the emoji from the demo name
            let name = demo
                .name()
                .split_once(' ')
                .map_or(demo.name(), |(_, name)| name);

            let mut harness = Harness::new(|ctx| {
                demo.show(ctx, &mut true);
            });

            let window = harness.node().children().next().unwrap();
            // TODO(lucasmerlin): Windows should probably have a label?
            //let window = harness.get_by_label(name);

            let size = window.raw_bounds().expect("window bounds").size();
            harness.set_size(Vec2::new(size.width as f32, size.height as f32));

            // Run the app for some more frames...
            harness.run_ok();

            let mut options = SnapshotOptions::default();
            // The Bézier Curve demo needs a threshold of 2.1 to pass on linux
            if name == "Bézier Curve" {
                options.threshold = 2.1;
            }

            let result = harness.try_snapshot_options(&format!("demos/{name}"), &options);
            if let Err(err) = result {
                errors.push(err.to_string());
            }
        }

        assert!(errors.is_empty(), "Errors: {errors:#?}");
    }
}
