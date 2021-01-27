use egui::{CtxRef, Resize, ScrollArea, Ui, Window};

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
struct Demos {
    open: Vec<bool>,

    #[cfg_attr(feature = "persistence", serde(skip))]
    demos: Vec<Box<dyn super::Demo>>,
}
impl Default for Demos {
    fn default() -> Self {
        let demos: Vec<Box<dyn super::Demo>> = vec![
            Box::new(super::dancing_strings::DancingStrings::default()),
            Box::new(super::drag_and_drop::DragAndDropDemo::default()),
            Box::new(super::font_book::FontBook::default()),
            Box::new(super::markdown_editor::MarkdownEditor::default()),
            Box::new(super::painting::Painting::default()),
            Box::new(super::scrolling::Scrolling::default()),
            Box::new(super::sliders::Sliders::default()),
            Box::new(super::widget_gallery::WidgetGallery::default()),
            Box::new(super::window_options::WindowOptions::default()),
            // Tests:
            Box::new(super::layout_test::LayoutTest::default()),
            Box::new(super::tests::IdTest::default()),
            Box::new(super::input_test::InputTest::default()),
        ];
        Self {
            open: vec![false; demos.len()],
            demos,
        }
    }
}
impl Demos {
    pub fn checkboxes(&mut self, ui: &mut Ui) {
        let Self { open, demos } = self;
        for (ref mut open, demo) in open.iter_mut().zip(demos.iter()) {
            ui.checkbox(open, demo.name());
        }
    }

    pub fn show(&mut self, ctx: &CtxRef) {
        let Self { open, demos } = self;
        open.resize(demos.len(), false); // Handle deserialization of old data.
        for (ref mut open, demo) in open.iter_mut().zip(demos.iter_mut()) {
            demo.show(ctx, open);
        }
    }
}

// ----------------------------------------------------------------------------

/// A menu bar in which you can select different demo windows to show.
#[derive(Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct DemoWindows {
    open_windows: OpenWindows,

    demo_window: super::DemoWindow,

    /// open, title, view
    demos: Demos,
}

impl DemoWindows {
    /// Show the app ui (menu bar and windows).
    /// `sidebar_ui` can be used to optionally show some things in the sidebar
    pub fn ui(&mut self, ctx: &CtxRef) {
        egui::SidePanel::left("side_panel", 200.0).show(ctx, |ui| {
            ui.heading("✒ egui demos");

            ui.separator();

            ScrollArea::auto_sized().show(ui, |ui| {
                ui.label("egui is an immediate mode GUI library written in Rust.");
                ui.add(
                    egui::Hyperlink::new("https://github.com/emilk/egui").text(" egui home page"),
                );

                ui.label("egui can be run on the web, or natively on 🐧");

                ui.separator();

                ui.heading("Windows:");
                ui.indent("windows", |ui| {
                    self.open_windows.checkboxes(ui);
                    self.demos.checkboxes(ui);
                });

                ui.separator();

                if ui.button("Organize windows").clicked() {
                    ui.ctx().memory().reset_areas();
                }
            });
        });

        egui::TopPanel::top("menu_bar").show(ctx, |ui| {
            show_menu_bar(ui);
        });

        self.windows(ctx);
    }

    /// Show the open windows.
    fn windows(&mut self, ctx: &CtxRef) {
        let Self {
            open_windows,
            demo_window,
            demos,
            ..
        } = self;

        Window::new("✨ Demo")
            .open(&mut open_windows.demo)
            .scroll(true)
            .show(ctx, |ui| {
                demo_window.ui(ui);
            });

        Window::new("🔧 Settings")
            .open(&mut open_windows.settings)
            .scroll(true)
            .show(ctx, |ui| {
                ctx.settings_ui(ui);
            });

        Window::new("🔍 Inspection")
            .open(&mut open_windows.inspection)
            .scroll(true)
            .show(ctx, |ui| {
                ctx.inspection_ui(ui);
            });

        Window::new("📝 Memory")
            .open(&mut open_windows.memory)
            .resizable(false)
            .show(ctx, |ui| {
                ctx.memory_ui(ui);
            });

        demos.show(ctx);

        self.resize_windows(ctx);
    }

    fn resize_windows(&mut self, ctx: &CtxRef) {
        let open = &mut self.open_windows.resize;

        Window::new("resizable")
            .open(open)
            .scroll(false)
            .resizable(true)
            .show(ctx, |ui| {
                ui.label("scroll:    NO");
                ui.label("resizable: YES");
                ui.label(crate::LOREM_IPSUM);
            });

        Window::new("resizable + embedded scroll")
            .open(open)
            .scroll(false)
            .resizable(true)
            .default_height(300.0)
            .show(ctx, |ui| {
                ui.label("scroll:    NO");
                ui.label("resizable: YES");
                ui.heading("We have a sub-region with scroll bar:");
                ScrollArea::auto_sized().show(ui, |ui| {
                    ui.label(crate::LOREM_IPSUM_LONG);
                    ui.label(crate::LOREM_IPSUM_LONG);
                });
                // ui.heading("Some additional text here, that should also be visible"); // this works, but messes with the resizing a bit
            });

        Window::new("resizable + scroll")
            .open(open)
            .scroll(true)
            .resizable(true)
            .default_height(300.0)
            .show(ctx, |ui| {
                ui.label("scroll:    YES");
                ui.label("resizable: YES");
                ui.label(crate::LOREM_IPSUM_LONG);
            });

        Window::new("auto_sized")
            .open(open)
            .auto_sized()
            .show(ctx, |ui| {
                ui.label("This window will auto-size based on its contents.");
                ui.heading("Resize this area:");
                Resize::default().show(ui, |ui| {
                    ui.label(crate::LOREM_IPSUM);
                });
                ui.heading("Resize the above area!");
            });
    }
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
struct OpenWindows {
    demo: bool,

    // egui stuff:
    settings: bool,
    inspection: bool,
    memory: bool,
    resize: bool,
}

impl Default for OpenWindows {
    fn default() -> Self {
        Self {
            demo: true,
            ..OpenWindows::none()
        }
    }
}

impl OpenWindows {
    fn none() -> Self {
        Self {
            demo: false,

            settings: false,
            inspection: false,
            memory: false,
            resize: false,
        }
    }

    fn checkboxes(&mut self, ui: &mut Ui) {
        let Self {
            demo,
            settings,
            inspection,
            memory,
            resize,
        } = self;
        ui.label("egui:");
        ui.checkbox(settings, "🔧 Settings");
        ui.checkbox(inspection, "🔍 Inspection");
        ui.checkbox(memory, "📝 Memory");
        ui.separator();
        ui.checkbox(demo, "✨ Demo");
        ui.separator();
        ui.checkbox(resize, "↔ Resize examples");
        ui.separator();
        ui.label("Misc:");
    }
}

fn show_menu_bar(ui: &mut Ui) {
    use egui::*;

    menu::bar(ui, |ui| {
        menu::menu(ui, "File", |ui| {
            if ui.button("Organize windows").clicked() {
                ui.ctx().memory().reset_areas();
            }
            if ui
                .button("Clear egui memory")
                .on_hover_text("Forget scroll, collapsing headers etc")
                .clicked()
            {
                *ui.ctx().memory() = Default::default();
            }
        });
    });
}
